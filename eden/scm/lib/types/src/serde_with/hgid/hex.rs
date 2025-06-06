/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

use serde::Serializer;

use crate::HgId;

/// Serde `serialize_with` function to serialize `HgId` as hex string.
pub fn serialize<S>(id: &HgId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex = id.to_hex();
    serializer.serialize_str(&hex)
}

// bytes::deserialize can decode hex.
pub use super::bytes::deserialize;
