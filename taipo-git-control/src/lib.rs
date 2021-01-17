/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

/*! `Taipo git control` provides a distributed key-value data store using the git version control system.

Data values are arbitrary blobs (byte slices), referred to as items
and keys are paths (also byte slices). Blobs are represented by
[`RepoOid`]s. Blobs are notified using
[`FanlingRepository::notify_blob`], which returns a `RepoOid`. A copy
of the original Blob can be retrieved using
`FanlingRepository::get_blob`.

As `Taipo git control` is based on git, [`FanlingRepository::fetch`]
can result in a merge conflict. It is the user's responsibility to
resolve this before any further use of the system, by applying the
necessary changes.

* FUTURE example tests; cover all main cases including merge conflicts

Some code copied from [here](https://zsiciarz.github.io/24daysofrust/book/vol2/day16.html)

FUTURE: test named branch works
 */

#[macro_use]
extern crate quick_error;

#[macro_use]
mod error;
mod repo;
#[macro_use]
mod shared;
#[cfg(test)]
mod test;

pub use crate::error::{NullResult, RepoError, RepoResult};
pub use crate::repo::{
    Conflict, ConflictList, FanlingRepository, MergeOutcome, RepoActionRequired,
};
pub use crate::shared::{
    Change, ChangeList, EntryDescr, ObjectOperation, RepoOid, RepoOptions, Tracer,
};
