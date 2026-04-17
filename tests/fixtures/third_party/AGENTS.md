# Third-Party Test Fixtures (`tests/fixtures/third_party/`)

This directory contains Git submodules of third-party repositories used as test fixtures for integration and end-to-end testing.

## Agent Guidelines
* **Read-Only:** Do NOT modify any files within these submodule directories. They represent external, unmanaged state.
* **Testing:** These repositories are to be used strictly as inputs to verify our transformations and logic.
* **Updates:** Updating these submodules should be done carefully via standard `git submodule` commands if required by a specific task, but their internal code should never be altered directly by AI agents.
