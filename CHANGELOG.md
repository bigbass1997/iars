# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)

## [Unreleased]
- Changed: Moved `Item` into separate module
- Changed: Moved identifier validation to `Item::new`
- Added: Size argument to `Item::upload_file`
- Added: `Item::list` retrieves a list of files in an item
- Added: `Item::download_file`
- Added: Expanded `Item` documentation, including details about potential errors
- Added: Searching for tasks based on specified criteria
- Added: Retrieval of a task's log

## [0.1.0] - 2023-12-30
- Initial release
  - IA Item file uploading
  - Authorization (S3 tokens)
  - Identifier validation