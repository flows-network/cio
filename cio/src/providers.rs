use anyhow::{bail, Result};
use async_trait::async_trait;
use log::info;

use crate::{
    companies::Company,
    configs::{Group, User},
    db::Database,
};

/// This trait defines how to implement a provider for a vendor that manages users
/// and groups.
#[async_trait]
pub trait ProviderOps<U, G> {
    /// Ensure the user exists and has the correct information.
    async fn ensure_user(&self, db: &Database, company: &Company, user: &User) -> Result<String>;

    /// Ensure the group exists and has the correct information.
    async fn ensure_group(&self, db: &Database, company: &Company, group: &Group) -> Result<()>;

    async fn check_user_is_member_of_group(&self, company: &Company, user: &User, group: &str) -> Result<bool>;

    async fn add_user_to_group(&self, company: &Company, user: &User, group: &str) -> Result<()>;

    async fn remove_user_from_group(&self, company: &Company, user: &User, group: &str) -> Result<()>;

    async fn list_provider_users(&self, company: &Company) -> Result<Vec<U>>;

    async fn list_provider_groups(&self, company: &Company) -> Result<Vec<G>>;

    async fn delete_user(&self, company: &Company, user: &User) -> Result<()>;

    async fn delete_group(&self, company: &Company, group: &Group) -> Result<()>;
}

#[async_trait]
impl ProviderOps<ramp_api::types::User, ()> for ramp_api::Client {
    async fn ensure_user(&self, db: &Database, _company: &Company, user: &User) -> Result<String> {
        // TODO: this is wasteful find another way to do this.
        let departments = self.departments().get_all().await?;

        // Invite the new ramp user.
        let mut ramp_user = ramp_api::types::PostUsersDeferredRequest {
            email: user.email.to_string(),
            first_name: user.first_name.to_string(),
            last_name: user.last_name.to_string(),
            phone: user.recovery_phone.to_string(),
            role: ramp_api::types::Role::BusinessUser,
            // Add the manager.
            direct_manager_id: user.manager(db).ramp_id,
            department_id: String::new(),
            location_id: String::new(),
        };

        // Set the department.
        // TODO: this loop is wasteful.
        for dept in departments {
            if dept.name == user.department {
                ramp_user.department_id = dept.id;
                break;
            }
        }

        // TODO: If the department for the user is not empty but we don't
        // have a Ramp department, create it.

        // Add the manager.
        let r = self.users().post_deferred(&ramp_user).await?;

        // TODO(should we?): Create them a card.

        Ok(r.id)
    }

    // Ramp does not have groups so this is a no-op.
    async fn ensure_group(&self, _db: &Database, _company: &Company, _group: &Group) -> Result<()> {
        Ok(())
    }

    // Ramp does not have groups so this is a no-op.
    async fn check_user_is_member_of_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<bool> {
        Ok(false)
    }

    // Ramp does not have groups so this is a no-op.
    async fn add_user_to_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    // Ramp does not have groups so this is a no-op.
    async fn remove_user_from_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    async fn list_provider_users(&self, _company: &Company) -> Result<Vec<ramp_api::types::User>> {
        self.users()
            .get_all(
                "", // department id
                "", // location id
            )
            .await
    }

    // Ramp does not have groups so this is a no-op.
    async fn list_provider_groups(&self, _company: &Company) -> Result<Vec<()>> {
        Ok(vec![])
    }

    async fn delete_user(&self, _company: &Company, _user: &User) -> Result<()> {
        // TODO: Suspend the user from Ramp.
        Ok(())
    }

    // Ramp does not have groups so this is a no-op.
    async fn delete_group(&self, _company: &Company, _group: &Group) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl ProviderOps<octorust::types::SimpleUser, octorust::types::Team> for octorust::Client {
    async fn ensure_user(&self, _db: &Database, company: &Company, user: &User) -> Result<String> {
        if user.github.is_empty() {
            // Return early, this user doesn't have a github handle.
            return Ok(String::new());
        }

        let role = if user.is_group_admin {
            octorust::types::OrgsSetMembershipUserRequestRole::Admin
        } else {
            octorust::types::OrgsSetMembershipUserRequestRole::Member
        };

        // Check if the user is already a member of the org.
        let user_exists = match self
            .orgs()
            .get_membership_for_user(&company.github_org, &user.github)
            .await
        {
            Ok(membership) => {
                if membership.role.to_string() == role.to_string() {
                    info!(
                        "user `{}` is already a member of the github org `{}` with role `{}`",
                        user.github, company.github_org, role
                    );

                    true
                } else {
                    false
                }
            }
            Err(e) => {
                // If the error is Not Found we need to add them.
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!(
                        "checking if user `{}` is a member of the github org `{}` failed: {}",
                        user.github,
                        company.github_org,
                        e
                    );
                }

                false
            }
        };

        if !user_exists {
            // We need to add the user to the org or update their role, do it now.
            self.orgs()
                .set_membership_for_user(
                    &company.github_org,
                    &user.github,
                    &octorust::types::OrgsSetMembershipUserRequest {
                        role: Some(role.clone()),
                    },
                )
                .await?;

            info!(
                "updated user `{}` as a member of the github org `{}` with role `{}`",
                user.github, company.github_org, role
            );
        }

        // Now we need to ensure our user is a member of all the correct groups.
        for group in &user.groups {
            let is_member = self.check_user_is_member_of_group(company, user, group).await?;

            if !is_member {
                // We need to add the user to the team or update their role, do it now.
                self.add_user_to_group(company, user, group).await?;
            }
        }

        // Get all the GitHub teams.
        let gh_teams = self.list_provider_groups(company).await?;

        // Iterate over all the teams and if the user is a member and should not
        // be, remove them from the team.
        for team in &gh_teams {
            if user.groups.contains(&team.slug) {
                // They should be in the team, continue.
                continue;
            }

            // Now we have a github team. The user should not be a member of it,
            // but we need to make sure they are not a member.
            let is_member = self.check_user_is_member_of_group(company, user, &team.slug).await?;

            // They are a member of the team.
            // We need to remove them.
            if is_member {
                self.remove_user_from_group(company, user, &team.slug).await?;
            }
        }

        // We don't need to store the user id, so just return an empty string here.
        Ok(String::new())
    }

    async fn ensure_group(&self, _db: &Database, company: &Company, group: &Group) -> Result<()> {
        // Check if the team exists.
        match self.teams().get_by_name(&company.github_org, &group.name).await {
            Ok(team) => {
                let parent_team_id = if let Some(parent) = team.parent { parent.id } else { 0 };

                self.teams()
                    .update_in_org(
                        &company.github_org,
                        &group.name,
                        &octorust::types::TeamsUpdateInOrgRequest {
                            name: group.name.to_string(),
                            description: group.description.to_string(),
                            parent_team_id,
                            permission: None, // This is depreciated, so just pass none.
                            privacy: Some(octorust::types::Privacy::Closed),
                        },
                    )
                    .await?;

                info!("updated group `{}` in github org `{}`", group.name, company.github_org);

                // Return early here.
                return Ok(());
            }
            Err(e) => {
                // If the error is Not Found we need to add the team.
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!(
                        "checking if team `{}` exists in github org `{}` failed: {}",
                        group.name,
                        company.github_org,
                        e
                    );
                }
            }
        }

        // Create the team.
        let team = octorust::types::TeamsCreateRequest {
            name: group.name.to_string(),
            description: group.description.to_string(),
            maintainers: Default::default(),
            privacy: Some(octorust::types::Privacy::Closed),
            permission: None, // This is depreciated, so just pass none.
            parent_team_id: 0,
            repo_names: group.repos.clone(),
        };

        self.teams().create(&company.github_org, &team).await?;

        info!("created group `{}` in github org `{}`", group.name, company.github_org);

        Ok(())
    }

    async fn check_user_is_member_of_group(&self, company: &Company, user: &User, group: &str) -> Result<bool> {
        if user.github.is_empty() {
            // Return early.
            return Ok(false);
        }

        let role = if user.is_group_admin {
            octorust::types::TeamMembershipRole::Maintainer
        } else {
            octorust::types::TeamMembershipRole::Member
        };

        match self
            .teams()
            .get_membership_for_user_in_org(&company.github_org, group, &user.github)
            .await
        {
            Ok(membership) => {
                if membership.role == role {
                    // We can return early, they have the right perms.
                    info!(
                        "user `{}` is already a member of the github team `{}` with role `{}`",
                        user.github, group, role
                    );
                    return Ok(true);
                }
            }
            Err(e) => {
                // If the error is Not Found we need to add them.
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!(
                        "checking if user `{}` is a member of the github team `{}` failed: {}",
                        user.github,
                        group,
                        e
                    );
                }
            }
        }

        Ok(false)
    }

    async fn add_user_to_group(&self, company: &Company, user: &User, group: &str) -> Result<()> {
        if user.github.is_empty() {
            // User does not have a github handle, return early.
            return Ok(());
        }

        let role = if user.is_group_admin {
            octorust::types::TeamMembershipRole::Maintainer
        } else {
            octorust::types::TeamMembershipRole::Member
        };

        // We need to add the user to the team or update their role, do it now.
        self.teams()
            .add_or_update_membership_for_user_in_org(
                &company.github_org,
                group,
                &user.github,
                &octorust::types::TeamsAddUpdateMembershipUserInOrgRequest {
                    role: Some(role.clone()),
                },
            )
            .await?;

        info!(
            "updated user `{}` as a member of the github team `{}` with role `{}`",
            user.github, group, role
        );

        Ok(())
    }

    async fn remove_user_from_group(&self, company: &Company, user: &User, group: &str) -> Result<()> {
        if user.github.is_empty() {
            // User does not have a github handle, return early.
            return Ok(());
        }

        self.teams()
            .remove_membership_for_user_in_org(&company.github_org, group, &user.github)
            .await?;

        info!("removed `{}` from github team `{}`", user.github, group);

        Ok(())
    }

    async fn list_provider_users(&self, company: &Company) -> Result<Vec<octorust::types::SimpleUser>> {
        // List all the users in the GitHub organization.
        self.orgs()
            .list_all_members(
                &company.github_org,
                octorust::types::OrgsListMembersFilter::All,
                octorust::types::OrgsListMembersRole::All,
            )
            .await
    }

    async fn list_provider_groups(&self, company: &Company) -> Result<Vec<octorust::types::Team>> {
        // List all the teams in the GitHub organization.
        self.teams().list_all(&company.github_org).await
    }

    async fn delete_user(&self, company: &Company, user: &User) -> Result<()> {
        if user.github.is_empty() {
            // Return early.
            return Ok(());
        }

        // Delete the user from the GitHub org.
        // Removing a user from this list will remove them from all teams and
        // they will no longer have any access to the organization’s repositories.
        self.orgs().remove_member(&company.github_org, &user.github).await?;

        info!(
            "deleted user `{}` from github org `{}`",
            user.github, company.github_org
        );

        Ok(())
    }

    async fn delete_group(&self, company: &Company, group: &Group) -> Result<()> {
        self.teams().delete_in_org(&company.github_org, &group.name).await?;

        info!("deleted group `{}` in github org `{}`", group.name, company.github_org);

        Ok(())
    }
}

#[async_trait]
impl ProviderOps<gsuite_api::types::User, gsuite_api::types::Group> for gsuite_api::Client {
    async fn ensure_user(&self, db: &Database, company: &Company, user: &User) -> Result<String> {
        // First get the user from gsuite.
        match self
            .users()
            .get(
                &user.email,
                gsuite_api::types::DirectoryUsersListProjection::Full,
                gsuite_api::types::ViewType::AdminView,
            )
            .await
        {
            Ok(u) => {
                // Update the user with the settings from the config for the user.
                let gsuite_user = crate::gsuite::update_gsuite_user(&u, user, false, company).await;

                self.users().update(&gsuite_user.id, &gsuite_user).await?;

                crate::gsuite::update_user_aliases(self, &gsuite_user, user.aliases.clone(), company).await?;

                // Add the user to their teams and groups.
                crate::gsuite::update_user_google_groups(self, user, company).await?;

                info!("updated user `{}` in GSuite", user.email);

                // Return the ID.
                return Ok(gsuite_user.id);
            }
            Err(e) => {
                // If the error is Not Found we need to add them.
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!("checking if user `{}` exists in GSuite failed: {}", user.email, e);
                }
            }
        }

        // Create the user.
        let u: gsuite_api::types::User = Default::default();

        // The last argument here tell us to create a password!
        // Make sure it is set to true.
        let gsuite_user = crate::gsuite::update_gsuite_user(&u, user, true, company).await;

        let new_gsuite_user = self.users().insert(&gsuite_user).await?;

        // Send an email to the new user.
        // Do this here in case another step fails.
        user.send_email_new_gsuite_user(db, &gsuite_user.password).await?;

        crate::gsuite::update_user_aliases(self, &gsuite_user, user.aliases.clone(), company).await?;

        crate::gsuite::update_user_google_groups(self, user, company).await?;

        info!("created user `{}` in GSuite", user.email);

        Ok(new_gsuite_user.id)
    }

    async fn ensure_group(&self, db: &Database, company: &Company, group: &Group) -> Result<()> {
        match self
            .groups()
            .get(&format!("{}@{}", &group.name, &company.gsuite_domain))
            .await
        {
            Ok(mut google_group) => {
                google_group.description = group.description.to_string();

                // Write the group aliases.
                let mut aliases: Vec<String> = Default::default();
                for alias in &group.aliases {
                    aliases.push(format!("{}@{}", alias, &company.gsuite_domain));
                }
                google_group.aliases = aliases;

                self.groups()
                    .update(&format!("{}@{}", group.name, company.gsuite_domain), &google_group)
                    .await?;

                crate::gsuite::update_group_aliases(self, &google_group).await?;

                // Update the groups settings.
                crate::gsuite::update_google_group_settings(db, group, company).await?;

                info!("updated group `{}` in GSuite", group.name);

                // Return early.
                return Ok(());
            }
            Err(e) => {
                // If the error is Not Found we need to add them.
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!("checking if group `{}` exists in GSuite failed: {}", group.name, e);
                }
            }
        }

        // Create the group.
        let mut g: gsuite_api::types::Group = Default::default();

        // TODO: Make this more DRY since it is repeated above as well.
        g.name = group.name.to_string();
        g.email = format!("{}@{}", group.name, company.gsuite_domain);
        g.description = group.description.to_string();

        // Write the group aliases.
        let mut aliases: Vec<String> = Default::default();
        for alias in &group.aliases {
            aliases.push(format!("{}@{}", alias, &company.gsuite_domain));
        }
        g.aliases = aliases;

        let new_group = self.groups().insert(&g).await?;

        crate::gsuite::update_group_aliases(self, &new_group).await?;

        // Update the groups settings.
        crate::gsuite::update_google_group_settings(db, group, company).await?;

        info!("created group `{}` in GSuite", group.name);

        Ok(())
    }

    async fn check_user_is_member_of_group(&self, company: &Company, user: &User, group: &str) -> Result<bool> {
        let role = if user.is_group_admin {
            "OWNER".to_string()
        } else {
            "MEMBER".to_string()
        };

        match self
            .members()
            .get(&format!("{}@{}", group, company.gsuite_domain), &user.email)
            .await
        {
            Ok(member) => {
                if member.role == role {
                    // They have the right permissions.
                    info!(
                        "user `{}` is already a member of the GSuite group `{}` with role `{}`",
                        user.email, group, role
                    );
                    return Ok(true);
                }
            }
            Err(e) => {
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!(
                        "checking if user `{}` is a member of the GSuite group `{}` failed: {}",
                        user.email,
                        group,
                        e
                    );
                }
            }
        }

        Ok(false)
    }

    async fn add_user_to_group(&self, company: &Company, user: &User, group: &str) -> Result<()> {
        let role = if user.is_group_admin {
            "OWNER".to_string()
        } else {
            "MEMBER".to_string()
        };

        let is_member = self.check_user_is_member_of_group(company, user, group).await?;
        if is_member {
            // Update the member of the group.
            self.members()
                .update(
                    &format!("{}@{}", group, company.gsuite_domain),
                    &user.email,
                    &gsuite_api::types::Member {
                        role: role.to_string(),
                        email: user.email.to_string(),
                        delivery_settings: "ALL_MAIL".to_string(),
                        etag: "".to_string(),
                        id: "".to_string(),
                        kind: "".to_string(),
                        status: "".to_string(),
                        type_: "".to_string(),
                    },
                )
                .await?;

            info!(
                "updated user `{}` membership to GSuite group `{}` with role `{}`",
                user.email, group, role
            );
        } else {
            // Create the member of the group.
            self.members()
                .insert(
                    &format!("{}@{}", group, company.gsuite_domain),
                    &gsuite_api::types::Member {
                        role: role.to_string(),
                        email: user.email.to_string(),
                        delivery_settings: "ALL_MAIL".to_string(),
                        etag: "".to_string(),
                        id: "".to_string(),
                        kind: "".to_string(),
                        status: "".to_string(),
                        type_: "".to_string(),
                    },
                )
                .await?;

            info!(
                "created user `{}` membership to GSuite group `{}` with role `{}`",
                user.email, group, role
            );
        }

        Ok(())
    }

    async fn remove_user_from_group(&self, company: &Company, user: &User, group: &str) -> Result<()> {
        self.members()
            .delete(&format!("{}@{}", group, company.gsuite_domain), &user.email)
            .await?;

        info!("removed user `{}` from GSuite group `{}`", user.email, group);
        Ok(())
    }

    async fn list_provider_users(&self, company: &Company) -> Result<Vec<gsuite_api::types::User>> {
        self.users()
            .list_all(
                &company.gsuite_account_id,
                &company.gsuite_domain,
                gsuite_api::types::Event::Noop,
                gsuite_api::types::DirectoryUsersListOrderBy::Email,
                gsuite_api::types::DirectoryUsersListProjection::Full,
                "", // query
                "", // show deleted
                gsuite_api::types::SortOrder::Ascending,
                gsuite_api::types::ViewType::AdminView,
            )
            .await
    }

    async fn list_provider_groups(&self, company: &Company) -> Result<Vec<gsuite_api::types::Group>> {
        self.groups()
            .list_all(
                &company.gsuite_account_id,
                &company.gsuite_domain,
                gsuite_api::types::DirectoryGroupsListOrderBy::Email,
                "", // query
                gsuite_api::types::SortOrder::Ascending,
                "", // user_key
            )
            .await
    }

    async fn delete_user(&self, _company: &Company, user: &User) -> Result<()> {
        // First get the user from gsuite.
        let mut gsuite_user = self
            .users()
            .get(
                &user.email,
                gsuite_api::types::DirectoryUsersListProjection::Full,
                gsuite_api::types::ViewType::AdminView,
            )
            .await?;

        // Set them to be suspended.
        gsuite_user.suspended = true;
        gsuite_user.suspension_reason = "No longer in config file.".to_string();

        // Update the user.
        self.users().update(&user.email, &gsuite_user).await?;

        info!("suspended user `{}` from gsuite", user.email);

        Ok(())
    }

    async fn delete_group(&self, company: &Company, group: &Group) -> Result<()> {
        self.groups()
            .delete(&format!("{}@{}", &group.name, &company.gsuite_domain))
            .await?;

        info!("deleted group `{}` from gsuite", group.name);

        Ok(())
    }
}

#[async_trait]
impl ProviderOps<okta::types::User, okta::types::Group> for okta::Client {
    async fn ensure_user(&self, db: &Database, company: &Company, user: &User) -> Result<String> {
        // Create the profile for the Okta user.
        let profile = okta::types::UserProfile {
            city: user.home_address_city.to_string(),
            cost_center: Default::default(),
            country_code: user.home_address_country_code.to_string(),
            department: user.department.to_string(),
            display_name: user.full_name(),
            division: Default::default(),
            email: user.email.to_string(),
            employee_number: Default::default(),
            first_name: user.first_name.to_string(),
            honorific_prefix: Default::default(),
            honorific_suffix: Default::default(),
            last_name: user.last_name.to_string(),
            locale: Default::default(),
            login: user.email.to_string(),
            manager: user.manager(db).email.to_string(),
            manager_id: Default::default(),
            middle_name: Default::default(),
            mobile_phone: user.recovery_phone.to_string(),
            nick_name: Default::default(),
            organization: company.name.to_string(),
            postal_address: user.home_address_formatted.to_string(),
            preferred_language: Default::default(),
            primary_phone: user.recovery_phone.to_string(),
            profile_url: Default::default(),
            second_email: user.recovery_email.to_string(),
            state: user.home_address_state.to_string(),
            street_address: format!("{}\n{}", user.home_address_street_1, user.home_address_street_2)
                .trim()
                .to_string(),
            timezone: Default::default(),
            title: Default::default(),
            user_type: Default::default(),
            zip_code: user.home_address_zipcode.to_string(),
        };

        // Try to get the user.
        let mut user_id = match self.user().get(&user.email.replace('@', "%40")).await {
            Ok(mut okta_user) => {
                // Update the Okta user.
                okta_user.profile = Some(profile);
                self.user()
                    .update(
                        &okta_user.id,
                        false, // strict
                        &okta_user,
                    )
                    .await?;

                okta_user.id
            }
            Err(e) => {
                if !e.to_string().contains("404") {
                    // Otherwise bail.
                    bail!("checking if user `{}` exists in Okta failed: {}", user.email, e);
                }

                String::new()
            }
        };

        if user_id.is_empty() {
            // Create the user.
            let okta_user = self
                .user()
                .create(
                    true,             // activate
                    false,            // provider
                    "changePassword", // next_login
                    &okta::types::CreateUserRequest {
                        credentials: None,
                        group_ids: Default::default(),
                        profile: Some(profile),
                        type_: None,
                    },
                )
                .await?;

            user_id = okta_user.id;
        }

        Ok(user_id)
    }

    async fn ensure_group(&self, _db: &Database, _company: &Company, group: &Group) -> Result<()> {
        // Try to find the group with the name.
        let results = self
            .group()
            .list_all(
                &group.name, // query
                "",          // search
                "",          // expand
            )
            .await?;

        for mut result in results {
            let mut profile = result.profile.unwrap();
            if profile.name == group.name {
                // We found the group let's update it if we should.
                if profile.description != group.description {
                    // Update the group.
                    profile.description = group.description.to_string();

                    result.profile = Some(profile);

                    self.group().update(&result.id, &result).await?;

                    info!("updated group `{}` in Okta", group.name);
                } else {
                    info!("existing group `{}` in Okta is up to date", group.name);
                }

                return Ok(());
            }
        }

        // The group did not exist, let's create it.
        self.group()
            .create(&okta::types::Group {
                embedded: None,
                links: None,
                created: None,
                id: String::new(),
                last_membership_updated: None,
                last_updated: None,
                object_class: Default::default(),
                type_: None,
                profile: Some(okta::types::GroupProfile {
                    name: group.name.to_string(),
                    description: group.description.to_string(),
                }),
            })
            .await?;

        info!("created group `{}` in Okta", group.name);

        Ok(())
    }

    async fn check_user_is_member_of_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<bool> {
        Ok(false)
    }

    async fn add_user_to_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    async fn remove_user_from_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    async fn list_provider_users(&self, _company: &Company) -> Result<Vec<okta::types::User>> {
        self.user()
            .list_all(
                "", // query
                "", // filter
                "", // search
                "", // sort by
                "", // sort order
            )
            .await
    }

    async fn list_provider_groups(&self, _company: &Company) -> Result<Vec<okta::types::Group>> {
        self.group()
            .list_all(
                "", // query
                "", // search
                "", // expand
            )
            .await
    }

    async fn delete_user(&self, _company: &Company, _user: &User) -> Result<()> {
        Ok(())
    }

    async fn delete_group(&self, _company: &Company, group: &Group) -> Result<()> {
        // Try to find the group with the name.
        let results = self
            .group()
            .list_all(
                &group.name, // query
                "",          // search
                "",          // expand
            )
            .await?;

        for result in results {
            let profile = result.profile.unwrap();
            if profile.name == group.name {
                // We found the group let's delete it.
                self.group().delete(&result.id).await?;
                return Ok(());
            }
        }

        Ok(())
    }
}

/*
 *
 * Keep as empty boiler plate for now.

#[async_trait]
impl ProviderOps<ramp_api::types::User, ()> for ramp_api::Client {
    async fn ensure_user(&self, _db: &Database, _company: &Company, _user: &User) -> Result<String> {
        Ok(String::new())
    }

    async fn ensure_group(&self, _db: &Database, _company: &Company, _group: &Group) -> Result<()> {
        Ok(())
    }

    async fn check_user_is_member_of_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<bool> {
        Ok(false)
    }

    async fn add_user_to_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    async fn remove_user_from_group(&self, _company: &Company, _user: &User, _group: &str) -> Result<()> {
        Ok(())
    }

    async fn list_provider_users(&self, _company: &Company) -> Result<Vec<ramp_api::types::User>> {
        Ok(vec![])
    }

    async fn list_provider_groups(&self, _company: &Company) -> Result<Vec<()>> {
        Ok(vec![])
    }

    async fn delete_user(&self, _company: &Company, _user: &User) -> Result<()> {
        Ok(())
    }

    async fn delete_group(&self, _company: &Company, _group: &Group) -> Result<()> {
        Ok(())
    }
}

*/