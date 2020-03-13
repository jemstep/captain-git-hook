use crate::error::CapnError;
use git2::Oid;
use std::error::Error;

pub enum ReferenceUpdate {
    New {
        new_commit_id: Oid,
        ref_name: String,
    },
    Delete {
        old_commit_id: Oid,
        ref_name: String,
    },
    Update {
        old_commit_id: Oid,
        new_commit_id: Oid,
        ref_name: String,
    },
}

impl ReferenceUpdate {
    pub fn from_git_hook_format(
        old_commit_id: &str,
        new_commit_id: &str,
        ref_name: &str,
    ) -> Result<ReferenceUpdate, Box<dyn Error>> {
        let old_commit_id = Oid::from_str(old_commit_id)?;
        let new_commit_id = Oid::from_str(new_commit_id)?;
        let ref_name = ref_name.to_owned();
        match (old_commit_id.is_zero(), new_commit_id.is_zero()) {
            (false, false) => Ok(ReferenceUpdate::Update {
                old_commit_id,
                new_commit_id,
                ref_name,
            }),
            (false, true) => Ok(ReferenceUpdate::Delete {
                old_commit_id,
                ref_name,
            }),
            (true, false) => Ok(ReferenceUpdate::New {
                new_commit_id,
                ref_name,
            }),
            (true, true) => Err(Box::new(CapnError::new("Invalid reference update specification, trying to update from a zero commit to another zero commit")))
        }
    }

    pub fn old_commit_id(&self) -> Option<Oid> {
        use self::ReferenceUpdate::*;
        match self {
            Delete { old_commit_id, .. } | Update { old_commit_id, .. } => Some(*old_commit_id),
            _ => None,
        }
    }

    pub fn new_commit_id(&self) -> Option<Oid> {
        use self::ReferenceUpdate::*;
        match self {
            New { new_commit_id, .. } | Update { new_commit_id, .. } => Some(*new_commit_id),
            _ => None,
        }
    }

    pub fn ref_name(&self) -> &str {
        use self::ReferenceUpdate::*;
        match self {
            New { ref_name, .. } | Delete { ref_name, .. } | Update { ref_name, .. } => ref_name,
        }
    }
}
