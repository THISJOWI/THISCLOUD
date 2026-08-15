# THISCLOUD Web UI Redesign Spec

## Design System Source
- Stitch project: `projects/4448210041066562096`
- Design system: `assets/15996705518239280238`
- Selected dashboard: "Cloud Infrastructure Dashboard" (screenshot ID: `249925a97d9f43748b3224c189fc0f6c`)

## Color Tokens (from Stitch design system)
- Background: `#0c1322`
- Surface: `#191f2f`
- Surface bright: `#323949`
- On surface: `#dce2f7`
- Primary: `#b4c5ff`
- Primary container: `#2563eb`
- Secondary: `#b7c8e1`
- Outline: `#8d90a0`
- Outline variant: `#434655`
- Success (running): `#22c55e`
- Warning (stopped): `#f59e0b`
- Error: `#ef4444`

## Typography
- Display: Inter 24px/600/32px, -0.02em tracking
- Headline: Inter 18px/600/24px
- Body MD: Inter 14px/400/20px
- Body SM: Inter 13px/400/18px
- Code SM: JetBrains Mono 12px/400/16px
- Label XS: Inter 11px/700/14px, 0.05em tracking

## Layout Structure
- Left sidebar: 260px fixed width, glass morphism
- Main content: fluid area with tabs and data grids
- Top bar: cluster status, user avatar
- Resource tree: nested navigation with status icons

## Component Specs

### Sidebar Navigation
- Glass panel with backdrop-filter: blur(20px) saturate(180%)
- THISCLOUD logo at top
- Tree navigation with 16px indentation per level
- Icons change color based on resource status
- Active state: primary blue accent

### Summary Cards
- 4-column grid: CPU Usage, Memory, Storage, Network
- Glass panels with 60-70% opacity
- Large metric number (display font)
- Secondary label (label-xs)
- Monospace for technical values

### Data Tables
- No hard borders (use spacing/background shifts)
- Hover state: surface bright
- Status badges: capsule-shaped with semantic colors
- Columns: Status, ID, Name, Node, CPU, RAM, Actions
- Actions: terminal (console), play/pause, etc.
- Monospace for IDs and technical data

### Status Badges
- Running: green bg at 10%, green text/border
- Stopped: amber bg at 10%, amber text/border
- Error: red bg at 10%, red text/border
- Capsule shape (border-radius: 9999px)

### Buttons
- Primary: solid blue (#2563eb), white text
- Secondary: ghost with subtle border
- Compact: 28px height for toolbars
- Full-rounded pills for CTAs

### Inputs
- Darker than panel surface (#111827)
- 1px border (#374151)
- Focus: 1px primary blue ring
- Monospace for technical input

## Pages to Implement

### 1. Dashboard (/admin)
- Summary cards: VMs count, CPU, RAM, Storage
- Recent VMs table
- System health indicator

### 2. Virtual Machines (/admin/vms)
- Full VM list with table
- Filters: status, node
- Bulk actions: start, stop, delete
- Create VM wizard (multi-step)

### 3. Networks (/admin/networks)
- Network list table
- Create network form
- Subnet/IP management

### 4. Storage (/admin/storage)
- Storage pools list
- Create storage form
- Disk usage visualization

### 5. Console (/admin/console)
- VM selector dropdown
- Terminal iframe with WebSocket
- Connection status indicator

### 6. VM Creation Wizard
- Step 1: Name, Node, OS Image
- Step 2: CPU, RAM, Disk
- Step 3: Network, Storage
- Step 4: Review & Create

## Bug Fixes to Include
1. `crypto.randomUUID()` → server-generated ID
2. Network/Storage as separate pages (not anchors)
3. Console WebSocket: `ws://127.0.0.1:8080/api/v1/vms/{id}/console/ws`
4. Console: add VM selector
5. `THISCLLOUD_WEB_PORT` typo fix (3 L's)
6. `middleware.ts` try-catch around `verifySessionToken()`
