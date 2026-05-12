# Jnoccio Fusion — Encrypted Source

This directory is encrypted with [git-crypt](https://github.com/AGWA/git-crypt).
Without the decryption key, files appear as binary ciphertext.

## For authorized developers / open-source users

1. **Obtain the decryption key** from a project maintainer (out-of-band).
2. **Install git-crypt**:
   ```bash
   # macOS
   brew install git-crypt

   # Ubuntu / Debian
   sudo apt-get install git-crypt

   # Arch
   pacman -S git-crypt
   ```
3. **Unlock the repository**:
   ```bash
   git-crypt unlock /path/to/jnoccio-fusion.key
   ```
   In the Jekko TUI, you can also select the locked `Jnoccio Fusion` model and
   enter the path to the same key file. The TUI unlock runs locally and never
   asks you to paste raw key contents.
4. Files under `jnoccio-fusion/` are now readable and editable as normal.
5. The TUI unlock creates `jnoccio-fusion/.env.jnoccio` from
   `.env.jnoccio.example` if it is missing. Fill in provider keys from
   `KEYS.md`; the file is ignored and must not be committed.
6. Commit and push normally — encryption happens transparently through the
   git clean/smudge filter defined in `.gitattributes`.

## For contributors without the key

You can clone, build, and contribute to all other parts of the repository.
The `jnoccio-fusion/` files will appear as binary data and cannot be read
or modified. This is by design.

## Important notes

- **Filenames are visible.** git-crypt encrypts file _contents_, not names.
- **Commit messages are visible.** Do not put sensitive information in commit messages.
- **The key must never be committed to Git.** It is distributed out-of-band only.
- **Old history remains decryptable** if the key is ever leaked. git-crypt does
  not support key rotation for existing history.

## Troubleshooting

### Files appear as binary after pulling
Run `git-crypt unlock /path/to/key`. The smudge filter will decrypt on checkout.

### CI fails with "PLAINTEXT DETECTED"
The git-crypt filter was not active when you committed. Fix with:
```bash
git-crypt unlock /path/to/key
git rm --cached jnoccio-fusion/<file>
git add jnoccio-fusion/<file>
git commit --amend
```

When your working tree is unlocked, use the index/blob verifier instead of the
working-tree check:

```bash
rtk tools/check-encrypted-paths.sh --index
```

This checks the staged/index blobs and also fails if
`jnoccio-fusion/.env.jnoccio` is tracked.

### Cannot build the Rust crate
`Cargo.toml` and all `.rs` source files are encrypted. You need the key to
compile jnoccio-fusion.

### Merge conflicts in encrypted files
Make sure `git-crypt unlock` has been run before merging. When unlocked, the
filter decrypts transparently and standard merge resolution works. If you see
binary conflict markers, unlock first then retry the merge.

### Adding a new file to jnoccio-fusion/
New files are automatically covered by the `.gitattributes` rule
`jnoccio-fusion/** filter=git-crypt diff=git-crypt`. Just `git add` as normal
after unlocking.

## Local Real-Key Proof

Authorized keyholders can run the local-only unlock test with:

```bash
JNOCCIO_GIT_CRYPT_KEY_PATH=/path/to/jnoccio-fusion.key rtk bun test test/local/jnoccio-unlock.local.test.ts
```

The test clones a fresh temporary copy, unlocks it with the key file, verifies
Jnoccio files parse, creates `.env.jnoccio` from placeholders, and runs the
index encryption checker. It skips in CI, without the env var, or without
`git-crypt`.
