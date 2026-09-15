# Monorepo Guidelines & Workflow Rules

This repository (`rust-playground`) is a **Monorepo Playground with Extraction Workflow** for prototyping, experimentation, and hobby projects. Projects start here and graduate to standalone repositories once they reach PoC or MVP maturity.

All contributors and AI agents must follow these rules.

---

## 1. Commit Isolation & Hygiene (Crucial for `git filter-repo`)

To ensure any project can be extracted into its own repository with 100% of its history and zero unrelated commits:

- **Single Project Per Commit**: Never commit changes to multiple projects within the same commit. Every commit must touch only **one** project directory under `projects/`.
- **Commit Message Prefixing**: Always prefix commit messages with the project name or scope:
  - `<project_name>: <description>` (e.g. `no_std_println: add panic handler and _start entry point`)
  - `workspace: <description>` (e.g. `workspace: add dev profile with panic=abort`)
- **Separate Workspace Commits**: Changes to root configuration files (`/Cargo.toml`, `/.vscode/`, `/README.md`, `/.agents/`) must be committed separately from project-specific code changes.
- **Linear History Preferred**: Avoid merge commits across multiple project features. Use rebase (`git rebase main` or `git pull --rebase`) to keep a clean, linear commit log per project.

---

## 2. Project Independence & Optional Workspace Membership

Workspace membership is **optional**, not mandatory. Projects can be run as workspace members or as completely standalone crates:

- **Workspace Members**:
  - Included in the root `Cargo.toml` `members = [...]` array.
  - Best for standard projects that benefit from a shared `target/` cache and unified `cargo test / check`.
  - Profile overrides (`[profile.dev]`, `[profile.release]`) must live in the root `Cargo.toml`.
- **Standalone Crates (Non-Workspace)**:
  - Some projects (e.g. `no_std`, bare-metal, embedded, custom targets, or projects needing their own `[profile.*]` settings like `panic = "abort"`) do not need to be part of the workspace.
  - To run a project completely outside the root workspace:
    - Omit it from `members = [...]` or add it to `exclude = [...]` in root `Cargo.toml`.
    - Alternatively, define an empty `[workspace]` table inside the project's own `Cargo.toml` to stop Cargo from walking up to the root workspace.
    - These crates maintain their own isolated `target/` folder and `Cargo.lock`.
- **Self-Contained Crates**:
  - Every project under `projects/` must have its own complete `Cargo.toml`.
- **No Cross-Project Relative Dependencies**:
  - Never reference other tinker projects via relative paths (`path = "../other-project"`) unless intentionally coupled. Each project must remain extractable as a standalone repository.

---

## 3. Resolving Commit & Merge Conflicts

When working on multiple projects or branches:

- **Root `Cargo.toml` & `Cargo.lock` Conflicts**:
  1. If two branches add workspace members or dependencies, resolve the conflict in root `Cargo.toml` by keeping both entries.
  2. For `Cargo.lock`, resolve trivial conflicts or restore `git checkout --ours Cargo.lock` (or `--theirs`) and run `cargo check` / `cargo generate-lockfile` to produce a consistent lockfile.
  3. Commit the workspace lockfile resolution as `workspace: resolve dependency lockfile conflict`.
- **Project File Conflicts**:
  - Since projects are isolated under separate folders (`projects/<name>/`), file conflicts should only occur if the same project was edited concurrently.
  - Never squash or merge changes across different project boundaries during conflict resolution.

---

## 4. Graduating Projects with `git filter-repo`

When a project reaches MVP/PoC status and is ready for its own repository:

### Step 1: Clone a Fresh Copy (Never run filter-repo in-place)
`git filter-repo` destructively rewrites git history. **Always clone to a separate location:**
```bash
git clone ~/git/rust-playground ~/git/<project-name>
cd ~/git/<project-name>
```

### Step 2: Extract the Subdirectory
Run `git filter-repo` specifying the relative path of the project folder:
```bash
# For a project directly under projects/
git filter-repo --subdirectory-filter projects/<project-name>

# Or for nested categories (e.g., projects/no_std/no_std_println)
git filter-repo --subdirectory-filter projects/no_std/no_std_println
```

*This transforms `projects/<project-name>/` into the new repository root `/`, discarding all commits that did not touch this project.*

### Step 3: Validate Standalone Build & History
```bash
# 1. Verify history contains only this project's commits
git log --oneline

# 2. Verify crate builds and passes tests as a standalone repo
cargo check
cargo test

# 3. Add new remote and push
git remote add origin git@github.com:username/<project-name>.git
git branch -M main
git push -u origin main
```

### Step 4: Record Graduation in Playground
Back in `rust-playground`:
```bash
cd ~/git/rust-playground

# Optional: Tag graduation point
git tag -a graduated/<project-name> -m "Graduated <project-name> to standalone repo"

# Remove from playground or keep archived
git rm -r projects/<path-to-project>
# Update root Cargo.toml members if needed
git commit -m "<project-name>: graduated to standalone repository"
```
