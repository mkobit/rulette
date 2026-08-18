## ADDED Requirements

### Requirement: Drift-aware output writes

When emitting a target file, the `transform` command SHALL compare the rendered content against the existing file at that path, if one exists, before writing.
If the existing file's content is byte-identical to the rendered content, Rulette SHALL skip the write and leave the file (including its modification time) untouched.
Rulette SHALL report each target's emission status as one of created, updated, or unchanged.
This requirement composes with the existing all-or-nothing multi-target emission requirement: drift comparison happens before a write is attempted, and does not change the atomicity or rollback behavior of a failed multi-target emission.
When an invocation writes to a target path that already existed (status updated), and a later target in the same invocation fails, Rulette SHALL restore that path's original content as part of rollback rather than deleting it.
If an existing target path cannot be read back as content (for example: not valid UTF-8, or a permission error on read), Rulette SHALL treat this as a hard error for the whole invocation and SHALL NOT write any target in this run.
If an existing target path is not a regular file (for example: a symlink or a directory), Rulette SHALL treat this as a hard error for the whole invocation and SHALL NOT read through or write through it.

#### Scenario: Unchanged target is not rewritten

- **WHEN** a target file already exists on disk with content identical to what this invocation would render
- **THEN** Rulette SHALL NOT write to that file
- **AND** SHALL report the target's status as unchanged.

#### Scenario: Changed target is rewritten

- **WHEN** a target file exists on disk with content different from what this invocation would render
- **THEN** Rulette SHALL write the new content to that file
- **AND** SHALL report the target's status as updated.

#### Scenario: New target is created

- **WHEN** a target file does not yet exist on disk
- **THEN** Rulette SHALL write the rendered content to that file
- **AND** SHALL report the target's status as created.

#### Scenario: Multiple targets report independent statuses in one invocation

- **WHEN** a single invocation emits several targets, some unchanged, some updated, and some newly created
- **THEN** Rulette SHALL report each target's own status independently
- **AND** SHALL only write the targets whose status is created or updated.

#### Scenario: Rollback restores an overwritten target's original content

- **WHEN** an invocation overwrites an existing target (status updated)
- **AND** a later target in the same invocation fails
- **THEN** Rulette SHALL restore the overwritten target's original content
- **AND** SHALL exit with a non-zero exit code.

#### Scenario: Unreadable existing target aborts before any writes

- **WHEN** a target path already exists but its content cannot be read back (not valid UTF-8, or a permission error)
- **THEN** Rulette SHALL exit with a non-zero exit code
- **AND** SHALL NOT write any target in that invocation.

#### Scenario: Non-regular-file existing target aborts before any writes

- **WHEN** a target path already exists as a symlink or a directory instead of a regular file
- **THEN** Rulette SHALL exit with a non-zero exit code
- **AND** SHALL NOT read through or write through that path
- **AND** SHALL NOT write any target in that invocation.

### Requirement: Check mode reports drift without writing

The `transform` command SHALL support a check mode in which no target files are written to disk.
In check mode, Rulette SHALL still compute drift status (created, updated, or unchanged) for every target as if writing were about to happen, and SHALL report that status to the user.
If any target would be created or updated, Rulette SHALL exit with a non-zero exit code. If every target is unchanged, Rulette SHALL exit with a zero exit code.
Check mode SHALL perform no filesystem mutation of any kind, including creating parent directories, in addition to not writing target file content.
Check mode SHALL require at least one target that resolves to a file path; if every requested target is standard output, Rulette SHALL exit with a non-zero exit code and an error explaining there is nothing on disk to check.

#### Scenario: Check mode with no drift succeeds without writing

- **WHEN** `transform` is run in check mode
- **AND** every target's rendered content matches what already exists on disk
- **THEN** Rulette SHALL NOT write any files
- **AND** SHALL exit with a zero exit code.

#### Scenario: Check mode with drift fails without writing

- **WHEN** `transform` is run in check mode
- **AND** at least one target's rendered content differs from what exists on disk (or the target does not yet exist)
- **THEN** Rulette SHALL NOT write any files
- **AND** SHALL report which targets would change
- **AND** SHALL exit with a non-zero exit code.

#### Scenario: Check mode does not create parent directories

- **WHEN** `transform` is run in check mode
- **AND** a target's parent directory does not yet exist on disk
- **THEN** Rulette SHALL NOT create that directory
- **AND** SHALL still report the target's status as if it would be created.

#### Scenario: Check mode with only stdout targets fails

- **WHEN** `transform` is run in check mode
- **AND** no target resolves to a file path (all targets are standard output)
- **THEN** Rulette SHALL exit with a non-zero exit code
- **AND** SHALL report that there is nothing on disk to check.
