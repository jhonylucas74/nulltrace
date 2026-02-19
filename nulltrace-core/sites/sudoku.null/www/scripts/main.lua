-- Sudoku game logic for sudoku.null
-- Editable cells are Inputs; fixed cells are disabled with bold styling.
-- Uses ui.get_value, ui.set_input_value, ui.set_disabled, ui.set_visible, ui.set_class.

player_name = ""
grid = {}       -- grid[r*9+c] = digit 1-9 or ""
fixed = {}      -- fixed[r*9+c] = true if pre-filled (not editable)

-- CSS class constants for normal and valid-group cells
local CLASS_NORMAL = "w-10 h-10 min-w-10 min-h-10 p-0 text-center text-lg font-medium border border-stone-300 rounded-none bg-stone-50 text-stone-900 disabled:font-bold disabled:bg-amber-100 disabled:text-stone-800"
local CLASS_GREEN  = "w-10 h-10 min-w-10 min-h-10 p-0 text-center text-lg font-medium border border-green-400 rounded-none bg-green-50 text-stone-900 disabled:font-bold disabled:bg-green-200 disabled:text-stone-800"

for i = 0, 80 do
  grid[i] = ""
  fixed[i] = false
end

-- Returns true if the group of 9 cells (by index) all have distinct digits 1-9.
function isGroupValid(indices)
  local seen = {}
  for _, idx in ipairs(indices) do
    local v = grid[idx]
    if v == "" or seen[v] then return false end
    seen[v] = true
  end
  return true
end

-- Checks every row, column, and 3x3 box; applies green or normal class to each cell.
function checkGroups()
  local green = {}  -- green[idx] = true when the cell belongs to at least one valid group

  -- 9 rows
  for r = 0, 8 do
    local idxs = {}
    for c = 0, 8 do idxs[#idxs+1] = r*9+c end
    if isGroupValid(idxs) then
      for _, i in ipairs(idxs) do green[i] = true end
    end
  end

  -- 9 columns
  for c = 0, 8 do
    local idxs = {}
    for r = 0, 8 do idxs[#idxs+1] = r*9+c end
    if isGroupValid(idxs) then
      for _, i in ipairs(idxs) do green[i] = true end
    end
  end

  -- 9 sub-grids (3x3 boxes)
  for br = 0, 2 do
    for bc = 0, 2 do
      local idxs = {}
      for dr = 0, 2 do
        for dc = 0, 2 do
          idxs[#idxs+1] = (br*3+dr)*9 + (bc*3+dc)
        end
      end
      if isGroupValid(idxs) then
        for _, i in ipairs(idxs) do green[i] = true end
      end
    end
  end

  -- Apply class to every cell
  for r = 0, 8 do
    for c = 0, 8 do
      local idx = r*9+c
      local cellId = "cell-" .. r .. "-" .. c
      if green[idx] then
        ui.set_class(cellId, CLASS_GREEN)
      else
        ui.set_class(cellId, CLASS_NORMAL)
      end
    end
  end
end

function startGame()
  local name = ui.get_value("player_name")
  if name and name ~= "" then
    player_name = name
  else
    player_name = "Player"
  end
  ui.set_visible("welcome", false)
  ui.set_visible("game", true)
  ui.set_text("greeting", "Hello, " .. player_name .. "! Let's play Sudoku.")
  initPuzzle()
end

function initPuzzle()
  for i = 0, 80 do
    grid[i] = ""
    fixed[i] = false
  end
  -- Pre-filled cells (fixed)
  local given = {
    [0] = "5", [1] = "3", [4] = "7", [9] = "6", [12] = "1", [13] = "9", [14] = "5",
    [19] = "9", [20] = "8", [25] = "6", [27] = "8", [31] = "6", [35] = "3"
  }
  for idx, val in pairs(given) do
    grid[idx] = val
    fixed[idx] = true
  end
  -- Also: [36] = "4", [39] = "8", [41] = "3", [44] = "1", [45] = "7", [53] = "2"
  grid[36] = "4"; grid[39] = "8"; grid[41] = "3"; grid[44] = "1"
  grid[45] = "7"; grid[53] = "2"; grid[55] = "6"; grid[60] = "6"
  grid[61] = "2"; grid[66] = "8"; grid[76] = "4"; grid[79] = "1"; grid[80] = "9"
  fixed[36] = true; fixed[39] = true; fixed[41] = true; fixed[44] = true
  fixed[45] = true; fixed[53] = true; fixed[55] = true; fixed[60] = true
  fixed[61] = true; fixed[66] = true; fixed[76] = true; fixed[79] = true; fixed[80] = true

  refreshGrid()
end

function refreshGrid()
  for r = 0, 8 do
    for c = 0, 8 do
      local idx = r * 9 + c
      local cellId = "cell-" .. r .. "-" .. c
      ui.set_input_value(cellId, grid[idx] or "")
      ui.set_disabled(cellId, fixed[idx])
    end
  end
  checkGroups()
end

function syncGrid()
  -- Read all cell values from form and emit patches so re-render shows them
  for r = 0, 8 do
    for c = 0, 8 do
      local idx = r * 9 + c
      if not fixed[idx] then
        local name = "cell-" .. r .. "-" .. c
        local val = ui.get_value(name)
        if val and val ~= "" then
          local digit = string.sub(val, 1, 1)
          if digit >= "1" and digit <= "9" then
            grid[idx] = digit
          else
            grid[idx] = ""
          end
        else
          grid[idx] = ""
        end
      end
    end
  end
  refreshGrid()  -- refreshGrid already calls checkGroups()
end

function newGame()
  ui.set_visible("game", false)
  ui.set_visible("welcome", true)
  player_name = ""
end
