# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.
#require slow

  $ . "${TEST_FIXTURES}/library.sh"

setup configuration
  $ setup_common_config "blob_sqlite"
  $ mononoke_testtool drawdag -R repo --derive-all <<'EOF'
  > A-B-C
  > # bookmark: C main
  > # extra: A example_extra "123\xff"
  > EOF
  A=c1c5eb4a15a4c71edae31c84f8b23ec5008ad16be07fba5b872fe010184b16ba
  B=749add4e33cf83fda6cce6f4fb4e3037a171dd8068acef09b336fd8ae027bf6f
  C=93cd0903625ea3162047e2699c2ea20d531b634df84180dbeeeb4b62f8afa8cd

  $ mononoke_admin fetch -R repo -i 93cd0903625ea3162047e2699c2ea20d531b634df84180dbeeeb4b62f8afa8cd
  BonsaiChangesetId: 93cd0903625ea3162047e2699c2ea20d531b634df84180dbeeeb4b62f8afa8cd
  Author: author
  Message: C
  FileChanges:
  	 ADDED/MODIFIED: C 896ad5879a5df0403bfc93fc96507ad9c93b31b11f3d0fa05445da7918241e5d
  
  $ mononoke_admin fetch -R repo -i c1c5eb4a15a4c71edae31c84f8b23ec5008ad16be07fba5b872fe010184b16ba --json | jq -S .
  {
    "author": "author",
    "author_date": "1970-01-01T00:00:00Z",
    "changeset_id": "c1c5eb4a15a4c71edae31c84f8b23ec5008ad16be07fba5b872fe010184b16ba",
    "committer": null,
    "committer_date": null,
    "file_changes": {
      "A": {
        "Change": {
          "copy_from": null,
          "inner": {
            "content_id": "eb56488e97bb4cf5eb17f05357b80108a4a71f6c3bab52dfcaec07161d105ec9",
            "file_type": "Regular",
            "git_lfs": "FullContent",
            "size": 1
          }
        }
      }
    },
    "hg_extra": {
      "example_extra": [
        49,
        50,
        51,
        255
      ]
    },
    "message": "A",
    "parents": []
  }

  $ mononoke_admin fetch -R repo -B main -p "" -k hg
  A 005d992c5dcf32993668f7cede29d296c494a5d9 regular
  B 35e7525ce3a48913275d7061dd9a867ffef1e34d regular
  C a2e456504a5e61f763f1a0b36a6c247c7541b2b3 regular

  $ mononoke_admin fetch -R repo -B main -p "" -k fsnode
  Summary:
  Simple-Format-SHA1: f8af839f2ffaa63aa251fafdbea413cb21ae9176
  Simple-Format-SHA256: 17ffd9c91c2ff10a13f8689b098fd41c90d0b45b0c14ad96eede1217b56418a5
  Children: 3 files (3), 0 dirs
  Descendants: 3 files (3)
  Children list:
  A eb56488e97bb4cf5eb17f05357b80108a4a71f6c3bab52dfcaec07161d105ec9 regular
  B 55662471e2a28db8257939b2f9a2d24e65b46a758bac12914a58f17dcde6905f regular
  C 896ad5879a5df0403bfc93fc96507ad9c93b31b11f3d0fa05445da7918241e5d regular

  $ mononoke_admin fetch -R repo -B main -p "A" -k fsnode
  File-Type: regular
  Size: 1
  Content-Id: eb56488e97bb4cf5eb17f05357b80108a4a71f6c3bab52dfcaec07161d105ec9
  Sha1: 6dcd4ce23d88e2ee9568ba546c007c63d9131c1b
  Sha256: 559aead08264d5795d3909718cdd05abd49572e84fe55590eef31a88a08fdffd
  Git-Sha1: 8c7e5a667f1b771847fe88c01c3de34413a1b220
  
  A
