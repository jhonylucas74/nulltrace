# Code / Markdown / List / Table

**Document**

Rich content components for documentation, reports, and structured data. Also includes Blockquote, Pre, and Details.

---

## Code

```ntml
language  string   - syntax highlight language (lua, ntml, yaml, etc.)
block     true|false - block-level (true) or inline (false, default)
class     string   - Tailwind utility classes
children  CDATA    - code content via <![CDATA[...]]>
```

```ntml
<!-- Block code -->
<Code language="lua" block="true" class="bg-zinc-800/90 border border-zinc-700 rounded-xl p-6 font-mono text-sm"><![CDATA[
  local x = 42
  print(x)
]]></Code>

<!-- Inline code within text -->
<Text text="Use " /><Code language="ntml">action</Code><Text text=" on Button." />
```

---

## Markdown

Renders CommonMark markdown. Use for documentation, changelogs, and rich text content loaded from files.

```ntml
<Markdown class="prose prose-zinc prose-invert max-w-none"><![CDATA[
# System Report

**Status:** Operational

All services are running normally. See the [dashboard](/dashboard) for details.

- 12 VMs active
- 0 critical alerts
- Uptime: 99.8%
]]></Markdown>
```

---

## List / ListItem

```ntml
<!-- Unordered list (default) -->
<List>
  <ListItem text="First item" />
  <ListItem text="Second item" />
  <ListItem text="Third item" />
</List>

<!-- Ordered list -->
<List ordered="true">
  <ListItem text="Open terminal" />
  <ListItem text="Run exploit" />
  <ListItem text="Exfiltrate data" />
</List>
```

---

## Table

```ntml
headers   string   - comma-separated column headers
rows      list     - array of row data arrays

<Table
  headers="IP,Hostname,Status"
  rows="10.0.1.50,api-server,Up|10.0.1.80,db-01,Down" />
```

---

## Details (collapsible)

```ntml
<Details summary="Advanced options">
  <Column gap="3" class="mt-3">
    <Text text="These are hidden by default." class="text-sm text-zinc-400" />
    <Select name="log_level" value="Info">
      <option label="Error" value="Error" />
      <option label="Warning" value="Warning" />
      <option label="Info" value="Info" />
      <option label="Debug" value="Debug" />
    </Select>
  </Column>
</Details>

<Details summary="View raw response">
  <Pre class="mt-2 text-xs font-mono text-zinc-400"><![CDATA[
    HTTP/1.1 200 OK
    Content-Type: application/json
    {"ok": true}
  ]]></Pre>
</Details>
```

---

## Blockquote / Pre

```ntml
<!-- Blockquote: indented quote styling -->
<Blockquote>
  <Text text="Security through obscurity is not security." class="italic text-zinc-400" />
</Blockquote>

<!-- Pre: preserves whitespace and line breaks -->
<Pre class="text-xs font-mono text-zinc-400 bg-zinc-900 p-4 rounded-lg"><![CDATA[
  10.0.1.1   router
  10.0.1.50  api-server
  10.0.1.100 ntml.org
]]></Pre>
```
