FlowFTP — DESIGN.md

Version: v0.1
Status: Draft
Platform: macOS (Primary)
Design Language: Apple Human Interface Guidelines + Linear + Raycast
Target Audience: Developers, DevOps, System Administrators, Power Users

⸻

Design Philosophy

FlowFTP is not another FTP client.

It is a Remote Workspace designed specifically for developers on macOS.

The goal is to remove the complexity and frustration that traditional FTP clients introduce.

Three design principles guide every decision:

* Calm
* Fast
* Predictable

Everything should feel native to macOS.

⸻

Product Vision

“The most beautiful and enjoyable open-source FTP/SFTP client for macOS.”

FlowFTP should feel like:

* Finder
* Raycast
* Linear
* VSCode

combined into one application.

⸻

Core UX Principles

1. Content First

The files are the product.

UI should never compete with the user’s files.

No giant toolbars.

No dozens of tiny icons.

Whitespace is intentional.

⸻

2. Progressive Disclosure

Beginners only see:

* Connect
* Browse
* Upload
* Download

Advanced users can discover:

* Sync
* Compare
* Batch Rename
* Terminal
* Command Palette

Nothing should overwhelm first-time users.

⸻

3. Keyboard First

Every action should be accessible by keyboard.

Examples:

⌘K

Open Command Palette

⌘⇧U

Upload

⌘⇧D

Download

⌘R

Reconnect

⌘T

Open Transfer Queue

⸻

4. Zero Fear

Dangerous actions always have preview.

Delete

Move

Sync

Overwrite

must always show:

Preview → Confirm → Execute

⸻

Visual Style

Personality

Elegant

Calm

Professional

Developer-centric

Premium

Never playful.

Never skeuomorphic.

⸻

Design Keywords

Minimal

Clean

Native

Focused

Modern

Breathing Space

⸻

Color Palette

Light

Background

#F6F7F8

Sidebar

#ECEEF1

Card

#FFFFFF

Border

#E5E7EB

Primary Blue

#2563EB

Success

#16A34A

Warning

#F59E0B

Danger

#DC2626

Primary Text

#111827

Secondary Text

#6B7280

⸻

Dark

Background

#111315

Panel

#181A1D

Border

#2A2E33

Primary Blue

#60A5FA

Text

#F8FAFC

Secondary

#94A3B8

⸻

Typography

Primary

SF Pro Display

Monospace

JetBrains Mono

Numbers

Tabular Numbers

⸻

Spacing System

4

8

12

16

24

32

48

No random spacing.

Everything follows the spacing scale.

⸻

Window Layout

┌──────────────────────────────────────────────────────────────┐
│ Toolbar                                                      │
├──────────┬───────────────────────┬───────────────────────────┤
│ Sidebar  │ Local                 │ Remote                    │
│          │                       │                           │
├──────────┴───────────────────────┴───────────────────────────┤
│ Transfer Queue                                             ▲ │
└──────────────────────────────────────────────────────────────┘

⸻

Sidebar

Width

260 px

Contains

Connections

Favorites

Recent

Tags

Pinned Servers

Transfer History

Settings

The sidebar behaves similarly to Finder.

⸻

Toolbar

Contains only the essentials.

Logo

Quick Search

Connect

Sync

View Switch

Nothing else.

⸻

File Panels

Local

Remote

Both share identical UI.

Each panel contains:

Breadcrumb

Navigation Buttons

Search

File List

Status Bar

Users should never need to “learn” two interfaces.

⸻

File Row

Contains

Icon

Filename

Size

Modified

Permissions

Owner

Hover actions

Quick Preview

Download

Upload

Reveal

⸻

Transfer Queue

Docked at bottom.

Collapsible.

Shows

Progress

Speed

ETA

Retry

Pause

Resume

Cancel

Completed transfers automatically collapse after 30 seconds.

⸻

Right Inspector

Hidden by default.

Opens when selecting a file.

Contains

Preview

Metadata

Permissions

History

Checksum

No modal windows.

⸻

Command Palette

Shortcut

⌘K

Supports

Connect Production

Upload Current Folder

Download Selected

Sync

Search Bookmarks

Jump Folder

Toggle Theme

Every major action should be searchable.

⸻

Connection Manager

Displays

Production

Staging

Development

NAS

Cloud

Each server shows

Color Dot

Latency

Protocol

Status

Last Connected

⸻

Smart Sync

One of FlowFTP’s signature features.

Shows

Upload

Download

Delete

Conflict

Skip

before execution.

Users always know what will happen.

⸻

Notifications

Never intrusive.

Small toast.

Bottom-right corner.

Example

✓ Upload Completed

✓ Connection Established

⚠ Sync Conflict

❌ Permission Denied

⸻

Empty States

Never empty.

Instead show helpful guidance.

“No recent connections”

↓

Connect to your first server

“No transfers”

↓

Upload or download files

⸻

Error Experience

Traditional FTP clients show

Permission denied

FlowFTP explains

Permission denied.

Your account does not have write permission to this directory.

Suggested actions

View Permissions

Retry

Open Parent Folder

⸻

Drag & Drop

Drag local → remote

Upload

Drag remote → local

Download

Drag between remote folders

Move

Feels identical to Finder.

⸻

Quick Preview

Images

Markdown

Text

JSON

YAML

PDF

No download required.

⸻

Accessibility

Keyboard navigation

VoiceOver support

High Contrast

Reduced Motion

Large Text

Fully accessible.

⸻

Motion Design

Animations

180–220 ms

Spring easing

Subtle only.

No bouncing.

No flashy effects.

⸻

Performance Goals

Launch

< 500 ms

Connect

< 2 seconds

Search

Instant

10,000 files

60 FPS scrolling

⸻

Future Features

Remote Terminal

SSH Session

Folder Compare

Git Status

Diff Viewer

Cloud Storage

S3

R2

Backblaze

Dropbox

Google Drive

Multiple Tabs

Workspace Profiles

Deploy Pipelines

Remote Code Editor

Extensions

Plugin Marketplace

⸻

UX Anti-Patterns

Never use modal dialogs unnecessarily.

Never ask users to confirm safe actions.

Never hide progress.

Never display raw protocol errors without explanation.

Never require users to open Settings just to connect.

Never expose technical jargon to beginners.

⸻

Final UX Goal

When users switch from Cyberduck or FileZilla, the first impression should be:

“This feels like Finder, but for servers.”

After one week of usage, the application should become invisible.

Users should think about their files—not the FTP client.