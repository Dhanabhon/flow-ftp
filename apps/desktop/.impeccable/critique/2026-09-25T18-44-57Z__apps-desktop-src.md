---
target: FlowFTP desktop app UI
total_score: 22
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 3
target_identity: "file:/Users/tom/Projects/GitHub/flow-ftp/apps/desktop/apps/desktop/src"
timestamp: 2026-09-25T18-44-57Z
slug: apps-desktop-src
---
# FlowFTP UX/UI Critique — apps/desktop (critique + technical audit)

Method: dual-agent (A: design review, B: detector/URL scan) + technical audit (C).

## Design Health Score: 22/40 (Acceptable — significant work needed)
| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | System status | 2 | No listing-load indicator, no connection status in header, dead Refresh |
| 2 | Match real world | 3 | Domain-correct copy; "Perm" jargon |
| 3 | User control & freedom | 2 | No undo; no retry on failed transfers |
| 4 | Consistency & standards | 2 | Nav switches nothing; click-deselect vs Finder |
| 5 | Error prevention | 2 | Profile delete unconfirmed; silent overwrite |
| 6 | Recognition over recall | 3 | Hover-only tooltips; dead More buttons |
| 7 | Flexibility & efficiency | 2 | Advertised palette shortcuts unbound |
| 8 | Aesthetic & minimalist | 3 | "Pro" badge decorative noise |
| 9 | Error recovery | 2 | Raw backend strings, 4s toasts |
| 10 | Help & documentation | 1 | No help entry; misleading empty state |

## Audit Health Score: 11/20 (Acceptable)
A11y 1/4 (unnamed icon buttons, invisible focus, no focus trap, contrast fails: fg-subtle dark 4.0:1, accent-fg 3.7:1, danger-muted 2.7:1) · Perf 2/4 (no virtualization, 10k rows, width transition on progress) · Responsive 3/4 (rows reserve 327px vs ~324px per pane at 1180px min) · Theming 3/4 (text-white x2, dead .glass) · Integrity 2/4 (nav facade, dead controls, mock in CommandPalette, activeConnectionId 'c1' seed leak)

## Design Specificity
Skeleton is genuinely FlowFTP (dual pane + inspector + keychain card, authored tokens). Shell dressed in interchangeable Raycast/Linear costume: Pro badge with no plan, unloaded Inter token, palette commands wired to nothing.

## Priority Issues
1. [P1] Sidebar is a broken promise — app.view only read for highlight; no views exist. Fix: collapse nav or build views.
2. [P1] Accessibility floor — unnamed icon buttons (tooltips visual-only), invisible focus (outline:none without replacement on rows/inputs/select), no focus trap/restore in modals, prefers-reduced-motion ignored, contrast fails on status pairs.
3. [P1] Disconnected state lies — remote pane shows "Empty directory" with no connection; 'c1' seed leaks into Tauri mode causing error footer. Fix: no-connection empty state + Connect CTA.
4. [P2] Dangerous-op asymmetry — profile delete unconfirmed; upload overwrite silent; delete confirm lacks host/folder context.
5. [P2] Keyboard contradiction — advertised shortcuts unbound; no arrow-key nav, no range select, hover-only tooltips; 7-button toolbar per pane (cognitive load failure).

Technical: no virtualization vs 10k goal; progress width->scaleX; breadcrumb min-w-0; failed-transfer Retry action; palette mock import -> ipc dual-mode.

## Detector evidence
Static scan: 0 findings (verified .svelte parsing gap; forced-HTML scan of all 17 components also 0). URL scan: 108 findings — 47 undersized-ui-text (10-11px), 21 low-contrast (agrees with computed WCAG failures), 21 ai-color-palette (FP), 6 layout-transition (progress width), 1 script-error (likely scan artifact, verify in webview). No user-visible overlay (browser bootstrap failed in subagent).

## Strengths
Token architecture (authored, theme-correct, FOUC-free); SyncPreview (Preview->Confirm->Execute + conflict resolution); primitive discipline + QuickConnect form labels.

## Personas
Alex: advertised shortcuts unbound; no keyboard file nav; cannot copy paths; click-deselects.
Jordan: dead nav ends; misleading "Empty directory"; unexplained Perm/Keychain/Pro.
Sam: delete confirm lacks host context; one-click credential delete; silent overwrite; no retry.

## Provocative questions
1. Is this a product with missing features, or a mock-up wearing a finished app's clothes?
2. What is calm about a 4-second failure toast?
3. Should a client that knows which server is production do more at 2am?
