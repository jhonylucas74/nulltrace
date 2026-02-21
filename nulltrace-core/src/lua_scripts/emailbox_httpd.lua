-- Emailbox NPC HTTP daemon: serves static files from /var/www and treats GET /inbox as a controller.
-- GET /inbox returns the first page of the mailbox inbox (formatted text), not a static file.
-- Run as: lua /var/www/emailbox_httpd.lua [root_path] (default /var/www).

local args = os.get_args()
local root = (args and args[1]) or "/var/www"

net.listen(80)
while true do
  local pkt = net.recv()
  if pkt and (tonumber(pkt.dst_port) or 0) == 80 then
    local body, status, headers
    local req_ok, req = pcall(http.parse_request, pkt.data)
    if not req_ok or not req then
      body = "# Bad request.\n"
      status = 400
      headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
    else
      -- Wrap handler in pcall so any error (e.g. httpd.serve) still gets a 500 response.
      local handler_ok, r1, r2, r3 = pcall(function()
        local path_only = http.normalize_request_path(req.path or "/")

        if path_only == "/inbox" or path_only == "/inbox/" then
          local entries = fs.ls("/etc/mail")
          if not entries or #entries < 1 then
            return "# No mail account.\n", 503
          end
          local address = entries[1].name
          local token = fs.read("/etc/mail/" .. address .. "/token")
          if not token or token == "" then
            return "# No mail token.\n", 503
          end
          local ok, r = pcall(mail.list, address, token, "inbox", 0)
          if not ok or not r or type(r) ~= "table" then
            return "# Failed to list inbox.\n", 502
          end
          local lines = { "# Received emails (first page)\n" }
          local emails = r.emails or {}
          for j = 1, #emails do
            local email = emails[j]
            if type(email) == "table" and email.id then
              lines[#lines + 1] = string.format(
                "from=%s to=%s subject=%s id=%s date=%s\n",
                email.from_address or "",
                email.to_address or "",
                email.subject or "",
                email.id,
                tostring(email.sent_at_ms or "")
              )
            end
          end
          return table.concat(lines, ""), 200
        else
          -- Root "/" or "" must map to "index" so httpd.serve finds /var/www/index
          local serve_path = path_only
          if serve_path == "/" or serve_path == "" then
            serve_path = "index"
          end
          return httpd.serve(root, serve_path)
        end
      end)

      if handler_ok and r1 then
        body = r1
        status = r2 or 500
        headers = r3
      else
        -- 500: include Lua error in response for debugging
        local err_msg = (type(r1) == "string" and r1 ~= "") and r1 or "# Server error.\n"
        body = "# Server error.\n\n" .. err_msg .. "\n"
        status = 500
      end
      if not headers then
        headers = { ["Content-Type"] = "text/plain; charset=utf-8" }
      end
    end

    local res = http.build_response(status, body or "", headers)
    net.send(pkt.src_ip, pkt.src_port, res)
  end
end
