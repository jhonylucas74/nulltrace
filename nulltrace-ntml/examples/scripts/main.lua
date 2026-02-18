-- scripts/main.lua
-- Exemplo de script Lua para NTML v0.2.0
-- Demonstra: API ui, API http, handlers de Button

-- Handler para o Button com action: "atualizarStatus"
function atualizarStatus()
  ui.set_disabled("btn-atualizar", true)
  ui.set_text("ultima-atualizacao", "Atualizando...")

  local res = http.get("/api/system/stats")

  if res.ok then
    ui.set_value("cpu-bar", res.data.cpu_percent)
    ui.set_value("ram-bar", res.data.ram_percent)
    ui.set_text("ultima-atualizacao", "Atualizado agora")
  else
    ui.set_text("ultima-atualizacao", "Erro ao atualizar")
  end

  ui.set_disabled("btn-atualizar", false)
end
