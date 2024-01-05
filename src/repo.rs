use std::path::Path;
use anyhow::{Context, Result};
use tracing::debug;

#[derive(Debug)]
pub struct BranchNameMeta {
    pub refname: String,
    pub local_name: String,
    pub upstream_name: Option<String>,
    pub oid: git2::Oid,
    pub summary: Option<String>,
}

pub struct Repo {
    repo: git2::Repository
}

fn normalize(s: impl Into<String>) -> Option<String> {
    Path::new(&s.into())
        .file_stem()
        .and_then(|s| s.to_str().map(ToOwned::to_owned))
}

impl Repo  {
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let path = crate::normalize::normalize_path(path)?;
        let repo = git2::Repository::discover(path)?;

        Ok(Self { repo })
    }

    pub fn get_name(&self, remote_name: Option<&str>) -> Result<String> {
        let remote = self.repo.find_remote(remote_name.unwrap_or("origin"))?;
        let url = remote.url().context("Remote url contains non-utf8 symbols")?;

        normalize(url).context("Cannot get repo name")
    }

    pub fn get_branch_name_meta(&self, branch_name: Option<&str>) -> Result<BranchNameMeta> {
        match branch_name {
            Some(_) => todo!(),
            None => {
                let head = self.repo.head()?;

                if !head.is_branch() {
                    anyhow::bail!("HEAD is not a branch");
                }

                let oid = head.target().context("HEAD is symbolic one")?;
                let summary = self.repo.find_commit(oid).ok().and_then(|c| c.summary().map(ToOwned::to_owned));

                let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

                let upstream_name: Option<String> = self.repo.branch_upstream_name(refname)
                    .ok()
                    .and_then(|b| b.as_str().map(ToOwned::to_owned))
                    .and_then(normalize)
                ;

                Ok(BranchNameMeta {
                    refname: refname.to_owned(),
                    local_name: normalize(refname).context("Non utf-8")?,
                    upstream_name,
                    oid,
                    summary,
                })
            }
        }
    }

    pub fn normalize_branch_name(&self, branch_name: Option<&str>) -> Result<String> {
        match branch_name {
            Some(bn) => Ok(bn.into()),
            None => {
                let head = self.repo.head()?;

                let refname = head.name().context("unable to get a branch name due to non-utf8 symbols")?;

                let r = self.repo.branch_upstream_name(refname)
                    .ok()
                    .and_then(|b| b.as_str().map(String::from))
                    .unwrap_or(refname.to_owned())
                ;

                normalize(&r).context("Cannot get branch name")
            }
        }
    }

    pub fn count_ahead_commits(&self) -> Result<usize> {
        let mut revwalk = self.repo.revwalk()?;
        // TODO: get default branch name
        revwalk.push_range("origin/master..HEAD")?;

        Ok(revwalk.count())
    }

    pub fn set_upstream(&self, local_name: &str, remote_name: &str, id: git2::Oid) -> Result<()> {
        let mut b = self.repo.find_branch(local_name, git2::BranchType::Local)?;

        self.repo.reference(
            format!("refs/remotes/origin/{remote_name}").as_str(),
            id,
            true,
            ""
        )?;

        b.set_upstream(Some(format!("origin/{remote_name}").as_str()))?;

        Ok(())
    }

    pub fn push(&self, refname: &str, remote_name: &str) -> Result<()> {
        let mut options = Self::get_push_options();

        let name = format!("{}:refs/heads/{}", refname, remote_name);

        dbg!(&name);

        self.repo.find_remote("origin")?.push(
            &[name.as_str()],
            Some(&mut options)
        )?;

        Ok(())
    }

    fn get_git_remote_callbacks() -> git2::RemoteCallbacks<'static> {
        let mut callbacks = git2::RemoteCallbacks::new();

        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
                username_from_url.unwrap_or_default(),
                None,
                Path::new(
                    &format!("{}/.ssh/id_rsa", std::env::var("HOME").unwrap())
                ),
                None,
            )
        });

        callbacks
    }

    fn get_push_options() -> git2::PushOptions<'static> {
        let mut callbacks = Self::get_git_remote_callbacks();

        callbacks.push_update_reference(|ref_name, status| {
            dbg!(ref_name, status);
            Ok(())
        });

        let mut options = git2::PushOptions::new();
        options.remote_callbacks(callbacks);

        options
    }

    pub fn status(&self) {
        todo!();
        // dbg!(self.repo.statuses(None));
    }

    pub fn create_and_switch(&self, branch_name: &str) -> Result<()> {
        if self.repo.find_branch(branch_name, git2::BranchType::Local).is_ok() {
            anyhow::bail!("Branch already exists");
        }

        let refs = self.repo.find_reference("refs/remotes/origin/master")?;
        let commit = refs.peel_to_commit()?;
        let branch = self.repo.branch(branch_name, &commit, false)?;
        if let Some(ref_name) = branch.into_reference().name() {
            self.repo.set_head(ref_name)?;
            self.repo.checkout_head(
                Some(
                    git2::build::CheckoutBuilder::default()
                        // For some reason the force is required to make the working directory actually get updated
                        // I suspect we should be adding some logic to handle dirty working directory states
                        // but this is just an example so maybe not.
                        .force(),
                )
            )?;
        } else {
            anyhow::bail!("Cannot switch to branch due invalid name");
        }

        Ok(())
    }

    pub fn fetch(&self, remote: Option<&str>) -> Result<()> {
        let remote = remote.unwrap_or("origin");
        let mut remote = self.repo
            .find_remote(remote)
            .or_else(|_| self.repo.remote_anonymous(remote))?;

        let mut cb = Self::get_git_remote_callbacks();

        cb.sideband_progress(|data| {
            debug!("remote: {}", std::str::from_utf8(data).unwrap());
            true
        });

        // This callback gets called for each remote-tracking branch that gets
        // updated. The message we output depends on whether it's a new one or an
        // update.
        cb.update_tips(|refname, a, b| {
            if a.is_zero() {
                debug!("[new]     {:20} {}", b, refname);
            } else {
                debug!("[updated] {:10}..{:10} {}", a, b, refname);
            }
            true
        });

        // Download the packfile and index it. This function updates the amount of
        // received data and the indexer stats which lets you inform the user about
        // progress.
        remote.download(&[] as &[&str], Some(git2::FetchOptions::new().remote_callbacks(cb)))?;

        {
            // If there are local objects (we got a thin pack), then tell the user
            // how many objects we saved from having to cross the network.
            let stats = remote.stats();
            if stats.local_objects() > 0 {
                debug!(
                    "\rReceived {}/{} objects in {} bytes (used {} local objects)",
                    stats.indexed_objects(),
                    stats.total_objects(),
                    stats.received_bytes(),
                    stats.local_objects()
                );
            } else {
                debug!(
                    "\rReceived {}/{} objects in {} bytes",
                    stats.indexed_objects(),
                    stats.total_objects(),
                    stats.received_bytes()
                );
            }
        }

        // Disconnect the underlying connection to prevent from idling.
        remote.disconnect()?;

        // Update the references in the remote's namespace to point to the right
        // commits. This may be needed even if there was no packfile to download,
        // which can happen e.g. when the branches have been changed but all the
        // needed objects are available locally.
        remote.update_tips(None, true, git2::AutotagOption::Unspecified, None)?;

        Ok(())
    }
}
