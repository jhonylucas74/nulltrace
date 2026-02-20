# Examples

Complete NTML snippets ready to copy and adapt. Each example shows real patterns used in NullTrace interfaces.

---

## Player status card

A compact status card showing player health, energy, and reputation with progress bars.

```ntml
<Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700 max-w-sm">
  <Column gap="4">
    <Row gap="3" align="center" justify="spaceBetween">
      <Row gap="2" align="center">
        <Icon name="user" size="18" class="text-amber-400" />
        <Text text="PLAYER STATUS" class="text-xs font-bold text-amber-400 uppercase tracking-wider" />
      </Row>
      <Badge text="Online" variant="success" />
    </Row>
    <Divider />
    <Column gap="3">
      <Row gap="3" align="center">
        <Text text="HP" class="text-xs text-zinc-400 w-8 font-mono" />
        <ProgressBar value="85" max="100" variant="success" class="flex-1 h-2 rounded-full" />
        <Text text="85" class="text-xs text-zinc-400 w-6 text-right font-mono" />
      </Row>
      <Row gap="3" align="center">
        <Text text="EN" class="text-xs text-zinc-400 w-8 font-mono" />
        <ProgressBar value="60" max="100" variant="default" class="flex-1 h-2 rounded-full" />
        <Text text="60" class="text-xs text-zinc-400 w-6 text-right font-mono" />
      </Row>
      <Row gap="3" align="center">
        <Text text="REP" class="text-xs text-zinc-400 w-8 font-mono" />
        <ProgressBar value="34" max="100" variant="warning" class="flex-1 h-2 rounded-full" />
        <Text text="34" class="text-xs text-zinc-400 w-6 text-right font-mono" />
      </Row>
    </Column>
  </Column>
</Container>
```

---

## Login form with Lua

A complete login form with validation and error feedback. Uses the full format with a script element.

```ntml
<head>
  <title>Login</title>
  <script src="scripts/auth.lua" />
</head>
<body>
  <Column gap="0" align="center" justify="center" class="min-h-screen bg-zinc-950 p-6">
    <Column id="form" gap="4" class="w-full max-w-sm p-6 bg-zinc-800 rounded-xl border border-zinc-700">
      <Column gap="1">
        <Heading level="2" text="Sign in" class="text-xl font-semibold text-zinc-100" />
        <Text text="Enter your credentials to continue." class="text-sm text-zinc-400" />
      </Column>
      <Input name="username" placeholder="Username" class="w-full p-2.5 rounded-lg bg-zinc-900 border border-zinc-600 text-zinc-200 text-sm" />
      <Input name="password" type="password" placeholder="Password" class="w-full p-2.5 rounded-lg bg-zinc-900 border border-zinc-600 text-zinc-200 text-sm" />
      <Text id="error-msg" text="" class="text-sm text-red-400 hidden" />
      <Button id="submit-btn" action="doLogin" variant="primary" class="w-full">
        <Text text="Sign in" class="font-medium" />
      </Button>
    </Column>
  </Column>
</body>
```

```lua
-- scripts/auth.lua
function doLogin()
  local user = ui.get_value("username")
  local pass = ui.get_value("password")
  if user == "" or pass == "" then
    ui.set_text("error-msg", "Please fill in all fields.")
    ui.set_visible("error-msg", true)
    return
  end
  ui.set_disabled("submit-btn", true)
  local res = http.post("/api/login", { username = user, password = pass })
  if res.ok then
    ui.set_visible("form", false)
  else
    ui.set_text("error-msg", res.error or "Login failed. Try again.")
    ui.set_visible("error-msg", true)
    ui.set_disabled("submit-btn", false)
  end
end
```

---

## Navigation sidebar

A sticky sidebar navigation with active state indicator. Ideal for multi-page applications.

```ntml
<Container class="w-52 border-r border-zinc-800 min-h-screen bg-zinc-900 p-4">
  <Column gap="1">
    <Row gap="2" align="center" class="mb-6 px-2">
      <Icon name="terminal" size="20" class="text-amber-400" />
      <Text text="NullOS" class="font-semibold text-zinc-100" />
    </Row>

    <Text text="NAVIGATION" class="text-xs font-semibold text-zinc-500 uppercase tracking-wider px-2 mb-2" />

    <!-- Active item -->
    <Link href="/dashboard">
      <Row gap="2" align="center" class="px-3 py-2 rounded-lg bg-amber-500/10 border border-amber-500/20">
        <Icon name="layout-dashboard" size="16" class="text-amber-400" />
        <Text text="Dashboard" class="text-sm font-medium text-amber-400" />
      </Row>
    </Link>

    <!-- Inactive items -->
    <Link href="/network">
      <Row gap="2" align="center" class="px-3 py-2 rounded-lg hover:bg-zinc-800 transition-colors duration-200">
        <Icon name="network" size="16" class="text-zinc-400" />
        <Text text="Network" class="text-sm text-zinc-400 hover:text-zinc-200" />
      </Row>
    </Link>
    <Link href="/files">
      <Row gap="2" align="center" class="px-3 py-2 rounded-lg hover:bg-zinc-800 transition-colors duration-200">
        <Icon name="folder" size="16" class="text-zinc-400" />
        <Text text="Files" class="text-sm text-zinc-400 hover:text-zinc-200" />
      </Row>
    </Link>
    <Link href="/logs">
      <Row gap="2" align="center" class="px-3 py-2 rounded-lg hover:bg-zinc-800 transition-colors duration-200">
        <Icon name="scroll-text" size="16" class="text-zinc-400" />
        <Text text="Logs" class="text-sm text-zinc-400 hover:text-zinc-200" />
      </Row>
    </Link>
  </Column>
</Container>
```

---

## Dashboard metric cards

A responsive grid of metric cards. Good for dashboards and status overview screens.

```ntml
<Column gap="6" class="p-8 bg-zinc-950 min-h-screen">
  <Row gap="3" align="center" justify="spaceBetween">
    <Heading level="1" text="System Overview" class="text-2xl font-bold text-zinc-100" />
    <Badge text="Live" variant="success" />
  </Row>
  <Grid columns="3" gap="4" class="w-full">
    <Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700">
      <Column gap="1">
        <Row gap="2" align="center">
          <Icon name="server" size="14" class="text-zinc-500" />
          <Text text="VMs Active" class="text-xs text-zinc-400 uppercase tracking-wider" />
        </Row>
        <Text text="12" class="text-3xl font-bold text-zinc-100 tabular-nums" />
        <Text text="+2 this session" class="text-xs text-green-400" />
      </Column>
    </Container>
    <Container class="p-5 bg-zinc-800 rounded-xl border border-zinc-700">
      <Column gap="1">
        <Row gap="2" align="center">
          <Icon name="zap" size="14" class="text-zinc-500" />
          <Text text="Scripts Run" class="text-xs text-zinc-400 uppercase tracking-wider" />
        </Row>
        <Text text="847" class="text-3xl font-bold text-zinc-100 tabular-nums" />
        <Text text="Last 24h" class="text-xs text-zinc-500" />
      </Column>
    </Container>
    <Container class="p-5 bg-zinc-800 rounded-xl border border-amber-500/20 bg-amber-500/5">
      <Column gap="1">
        <Row gap="2" align="center">
          <Icon name="alert-triangle" size="14" class="text-amber-400" />
          <Text text="Alerts" class="text-xs text-zinc-400 uppercase tracking-wider" />
        </Row>
        <Text text="3" class="text-3xl font-bold text-amber-400 tabular-nums" />
        <Text text="Require attention" class="text-xs text-amber-400/70" />
      </Column>
    </Container>
  </Grid>
</Column>
```

---

## Notification panel

A list of notifications with icons, timestamps, and dismiss actions.

```ntml
<Column gap="0" class="bg-zinc-900 rounded-xl border border-zinc-700 overflow-hidden max-w-sm">
  <Row gap="3" align="center" justify="spaceBetween" class="px-4 py-3 border-b border-zinc-800">
    <Text text="Notifications" class="text-sm font-semibold text-zinc-100" />
    <Badge text="3" variant="danger" />
  </Row>
  <Column gap="0">
    <Container class="px-4 py-3 border-b border-zinc-800/60 hover:bg-zinc-800/40 transition-colors duration-200">
      <Row gap="3" align="start">
        <Icon name="shield-alert" size="16" class="text-red-400 mt-0.5 flex-shrink-0" />
        <Column gap="0.5">
          <Text text="Intrusion detected" class="text-sm font-medium text-zinc-100" />
          <Text text="Port 22 scan from 192.168.5.14" class="text-xs text-zinc-400" />
          <Text text="2 min ago" class="text-xs text-zinc-600" />
        </Column>
      </Row>
    </Container>
    <Container class="px-4 py-3 border-b border-zinc-800/60 hover:bg-zinc-800/40 transition-colors duration-200">
      <Row gap="3" align="start">
        <Icon name="download" size="16" class="text-green-400 mt-0.5 flex-shrink-0" />
        <Column gap="0.5">
          <Text text="Download complete" class="text-sm font-medium text-zinc-100" />
          <Text text="exploit_pack_v3.zip (14 MB)" class="text-xs text-zinc-400" />
          <Text text="8 min ago" class="text-xs text-zinc-600" />
        </Column>
      </Row>
    </Container>
    <Container class="px-4 py-3 hover:bg-zinc-800/40 transition-colors duration-200">
      <Row gap="3" align="start">
        <Icon name="info" size="16" class="text-blue-400 mt-0.5 flex-shrink-0" />
        <Column gap="0.5">
          <Text text="VM restarted" class="text-sm font-medium text-zinc-100" />
          <Text text="api-server (10.0.1.50) is back online" class="text-xs text-zinc-400" />
          <Text text="15 min ago" class="text-xs text-zinc-600" />
        </Column>
      </Row>
    </Container>
  </Column>
</Column>
```

---

## Settings form

A settings panel with grouped options, select dropdowns, checkboxes, and a save button.

```ntml
<Column gap="6" class="max-w-lg p-6 bg-zinc-900 rounded-xl border border-zinc-700">
  <Heading level="2" text="Settings" class="text-xl font-semibold text-zinc-100" />

  <Column gap="4">
    <Text text="APPEARANCE" class="text-xs font-semibold text-zinc-500 uppercase tracking-widest" />
    <Column gap="2">
      <Text text="Theme" class="text-sm text-zinc-300" />
      <Select name="theme" value="System" class="w-full p-2.5 rounded-lg bg-zinc-800 border border-zinc-600 text-zinc-200 text-sm">
      <option label="Dark" value="Dark" />
      <option label="Light" value="Light" />
      <option label="System" value="System" />
    </Select>
    </Column>
    <Column gap="2">
      <Text text="Font size" class="text-sm text-zinc-300" />
      <Select name="font_size" value="Medium" class="w-full p-2.5 rounded-lg bg-zinc-800 border border-zinc-600 text-zinc-200 text-sm">
      <option label="Small" value="Small" />
      <option label="Medium" value="Medium" />
      <option label="Large" value="Large" />
    </Select>
    </Column>
  </Column>

  <Divider />

  <Column gap="4">
    <Text text="NOTIFICATIONS" class="text-xs font-semibold text-zinc-500 uppercase tracking-widest" />
    <Checkbox name="notif_intrusion" label="Intrusion alerts" />
    <Checkbox name="notif_downloads" label="Download completion" />
    <Checkbox name="notif_vm" label="VM status changes" />
  </Column>

  <Row gap="3" justify="end">
    <Button variant="ghost">
      <Text text="Cancel" class="text-sm text-zinc-400" />
    </Button>
    <Button action="saveSettings" variant="primary">
      <Text text="Save changes" class="text-sm font-medium" />
    </Button>
  </Row>

  <Text id="save-msg" text="" class="text-sm text-green-400" />
</Column>
```

---

## Data table

A styled data table showing connected hosts with status badges. Use for logs, scan results, and inventories.

```ntml
<Container class="rounded-xl border border-zinc-700 overflow-hidden">
  <!-- Header row -->
  <Container class="bg-zinc-800/80 border-b border-zinc-700 px-4 py-2.5">
    <Row gap="0">
      <Text text="IP Address" class="text-xs font-semibold text-zinc-400 uppercase tracking-wider flex-1" />
      <Text text="Hostname" class="text-xs font-semibold text-zinc-400 uppercase tracking-wider flex-1" />
      <Text text="OS" class="text-xs font-semibold text-zinc-400 uppercase tracking-wider w-28" />
      <Text text="Status" class="text-xs font-semibold text-zinc-400 uppercase tracking-wider w-20 text-right" />
    </Row>
  </Container>
  <!-- Data rows -->
  <Container class="px-4 py-2.5 border-b border-zinc-800/60 hover:bg-zinc-800/30 transition-colors duration-200">
    <Row gap="0" align="center">
      <Text text="10.0.1.100" class="text-sm font-mono text-zinc-300 flex-1" />
      <Text text="ntml.org" class="text-sm text-zinc-400 flex-1" />
      <Text text="NullOS 1.0" class="text-sm text-zinc-400 w-28" />
      <Container class="w-20 flex justify-end">
        <Badge text="Up" variant="success" />
      </Container>
    </Row>
  </Container>
  <Container class="px-4 py-2.5 border-b border-zinc-800/60 hover:bg-zinc-800/30 transition-colors duration-200">
    <Row gap="0" align="center">
      <Text text="10.0.1.50" class="text-sm font-mono text-zinc-300 flex-1" />
      <Text text="api-server" class="text-sm text-zinc-400 flex-1" />
      <Text text="NullOS 1.0" class="text-sm text-zinc-400 w-28" />
      <Container class="w-20 flex justify-end">
        <Badge text="Up" variant="success" />
      </Container>
    </Row>
  </Container>
  <Container class="px-4 py-2.5 hover:bg-zinc-800/30 transition-colors duration-200">
    <Row gap="0" align="center">
      <Text text="10.0.1.200" class="text-sm font-mono text-zinc-300 flex-1" />
      <Text text="unknown" class="text-sm text-zinc-500 flex-1" />
      <Text text="-" class="text-sm text-zinc-600 w-28" />
      <Container class="w-20 flex justify-end">
        <Badge text="Down" variant="danger" />
      </Container>
    </Row>
  </Container>
</Container>
```

---

## Full-page app with imports

A complete multi-section application using the full format with font, script, and component imports.

```ntml
<head>
  <title>Hack Terminal v2</title>
  <description>Advanced system control panel</description>
  <font family="JetBrains Mono" weights="400,700" />
  <script src="scripts/terminal.lua" />
  <script src="scripts/network.lua" />
  <import src="components/header.ntml" as="Header" />
  <import src="components/target-card.ntml" as="TargetCard" />
</head>
<body>
  <Column class="min-h-screen bg-zinc-950 font-[JetBrains_Mono]">
    <Header title="Hack Terminal" version="v2.0" />
    <Row gap="0" class="flex-1">
      <!-- Left: target list -->
      <Column gap="3" class="w-64 border-r border-zinc-800 p-4 bg-zinc-900">
        <Text text="TARGETS" class="text-xs font-bold text-zinc-500 uppercase tracking-widest" />
        <Input name="scan_ip" placeholder="192.168.x.x" class="w-full p-2 rounded bg-zinc-800 border border-zinc-700 text-sm text-zinc-200" />
        <Button action="scanTarget" variant="secondary" class="w-full">
          <Text text="Scan" class="text-sm font-medium" />
        </Button>
        <Divider />
        <TargetCard ip="10.0.1.50" hostname="api-server" status="vulnerable" />
        <TargetCard ip="10.0.1.80" hostname="db-01" status="patched" />
      </Column>
      <!-- Right: output terminal -->
      <Column gap="3" class="flex-1 p-6">
        <Text text="OUTPUT" class="text-xs font-bold text-zinc-500 uppercase tracking-widest" />
        <Container id="terminal-output" class="flex-1 p-4 bg-zinc-900 rounded-lg border border-zinc-800 min-h-64">
          <Text text="Ready." class="text-sm text-green-400" />
        </Container>
        <Row gap="3">
          <Input name="cmd_input" placeholder="Enter command..." class="flex-1 p-2.5 rounded-lg bg-zinc-800 border border-zinc-700 text-sm text-zinc-200 font-mono" />
          <Button action="runCommand" variant="primary">
            <Text text="Run" class="text-sm font-medium px-2" />
          </Button>
        </Row>
      </Column>
    </Row>
  </Column>
</body>
```
