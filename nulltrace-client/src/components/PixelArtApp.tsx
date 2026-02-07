import { useState, useCallback, useRef, useMemo, useEffect } from "react";
import { Pencil, Eraser, Hand, PaintBucket, ChevronDown } from "lucide-react";
import { useFilePicker } from "../contexts/FilePickerContext";
import { getDefaultInitialPath } from "../contexts/FilePickerContext";
import { useWindowManager } from "../contexts/WindowManagerContext";
import { PIXELART_EDITOR_SIZE } from "../contexts/WindowManagerContext";
import { createFile, setFileContent } from "../lib/fileSystem";
import {
  type PixelArtData,
  CANVAS_SIZES,
  PALETTES,
  createEmptyData,
  serializePixelArt,
  renderPixelArtToDataUrl,
} from "../lib/pixelArt";
import { HexColorPicker } from "react-colorful";
import Modal from "./Modal";
import styles from "./PixelArtApp.module.css";

function joinPath(base: string, name: string): string {
  const b = base.replace(/\/$/, "");
  return b ? `${b}/${name}` : `/${name}`;
}

const CELL_SIZE_PX = 14;
const MIN_ZOOM = 1;
const MAX_ZOOM = 12;

type Tool = "pencil" | "eraser" | "hand" | "fill";

interface Layer {
  id: string;
  name: string;
  visible: boolean;
  pixels: (string | null)[][];
}

interface PixelArtAppProps {
  windowId?: string;
}

export default function PixelArtApp({ windowId }: PixelArtAppProps) {
  const { resize, move } = useWindowManager();
  const [data, setData] = useState<PixelArtData | null>(null);
  const [layers, setLayers] = useState<Layer[]>([]);
  const [activeLayerId, setActiveLayerId] = useState<string | null>(null);
  const [selectedPaletteId, setSelectedPaletteId] = useState(PALETTES[0].id);
  const [selectedColor, setSelectedColor] = useState(PALETTES[0].colors[0]);
  const [tool, setTool] = useState<Tool>("pencil");
  const [isSpacePressed, setIsSpacePressed] = useState(false);
  const [zoom, setZoom] = useState(1.5);
  const [previewZoom, setPreviewZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [saveModalOpen, setSaveModalOpen] = useState(false);
  const [saveFolderPath, setSaveFolderPath] = useState<string | null>(null);
  const [saveFilename, setSaveFilename] = useState("pixel-art.json");
  const [saveError, setSaveError] = useState("");
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [previewUrl, setPreviewUrl] = useState<string>("");
  const isDrawingRef = useRef(false);
  const isPanningRef = useRef(false);
  const panStartRef = useRef({ x: 0, y: 0 });
  const pointerStartRef = useRef({ x: 0, y: 0 });
  const gridRef = useRef<HTMLDivElement | null>(null);
  const viewportRef = useRef<HTMLDivElement | null>(null);
  const menuBarRef = useRef<HTMLDivElement | null>(null);
  const [fileMenuOpen, setFileMenuOpen] = useState(false);
  const [viewMenuOpen, setViewMenuOpen] = useState(false);
  const { openFilePicker } = useFilePicker();
  const activeTool = isSpacePressed ? "hand" : tool;
  const previewTimerRef = useRef<number | null>(null);

  const startNewArt = useCallback(
    (size: number) => {
      const next = createEmptyData(size, size);
      const baseLayer: Layer = {
        id: `layer-${Date.now()}`,
        name: "Layer 1",
        visible: true,
        pixels: next.pixels.map((row) => row.map(() => null)),
      };
      setData(next);
      setLayers([baseLayer]);
      setActiveLayerId(baseLayer.id);
      setZoom(1.5);
      setPan({ x: 0, y: 0 });
      if (windowId && typeof window !== "undefined") {
        const w = PIXELART_EDITOR_SIZE.width;
        const h = PIXELART_EDITOR_SIZE.height;
        resize(windowId, w, h);
        const dockBottom = 20;
        const dockHeight = 56;
        const safeBottom = dockBottom + dockHeight;
        const availableHeight = window.innerHeight - safeBottom;
        const centerX = Math.max(0, (window.innerWidth - w) / 2);
        const centerY = Math.max(0, Math.min((availableHeight - h) / 2, availableHeight - h));
        move(windowId, centerX, centerY);
      }
    },
    [windowId, resize, move]
  );

  const currentPalette = useMemo(
    () => PALETTES.find((p) => p.id === selectedPaletteId) ?? PALETTES[0],
    [selectedPaletteId]
  );

  // When switching palette, snap selected color to the new palette only if it is not in the new palette.
  // Intentionally not depending on selectedColor so the custom picker can change color without resetting.
  useEffect(() => {
    setSelectedColor((prev) =>
      currentPalette.colors.includes(prev) ? prev : currentPalette.colors[0]
    );
  }, [selectedPaletteId, currentPalette]);

  const activeLayerIndex = useMemo(
    () => layers.findIndex((l) => l.id === activeLayerId),
    [layers, activeLayerId]
  );

  const compositePixels = useMemo(() => {
    if (!data) return [];
    const pixels: string[][] = [];
    for (let y = 0; y < data.height; y++) {
      pixels[y] = [];
      for (let x = 0; x < data.width; x++) {
        let color: string | null = null;
        for (let i = layers.length - 1; i >= 0; i--) {
          const layer = layers[i];
          if (!layer.visible) continue;
          const value = layer.pixels[y]?.[x] ?? null;
          if (value) {
            color = value;
            break;
          }
        }
        pixels[y][x] = color ?? "#ffffff";
      }
    }
    return pixels;
  }, [data, layers]);

  useEffect(() => {
    if (!data) return;
    if (previewTimerRef.current) {
      window.clearTimeout(previewTimerRef.current);
    }
    previewTimerRef.current = window.setTimeout(() => {
      const previewData: PixelArtData = {
        width: data.width,
        height: data.height,
        pixels: compositePixels,
      };
      const url = renderPixelArtToDataUrl(previewData);
      setPreviewUrl(url);
    }, 180);
    return () => {
      if (previewTimerRef.current) window.clearTimeout(previewTimerRef.current);
    };
  }, [data, compositePixels]);

  const setPixel = useCallback(
    (x: number, y: number) => {
      if (!data || activeLayerIndex < 0) return;
      setLayers((prev) => {
        const next = prev.map((layer, idx) => {
          if (idx !== activeLayerIndex) return layer;
          const updated = layer.pixels.map((row, rowIndex) =>
            rowIndex === y
              ? row.map((cell, colIndex) =>
                  colIndex === x ? (tool === "eraser" ? null : selectedColor) : cell
                )
              : [...row]
          );
          return { ...layer, pixels: updated };
        });
        return next;
      });
    },
    [data, activeLayerIndex, selectedColor, tool]
  );

  const floodFill = useCallback(
    (startX: number, startY: number) => {
      if (!data || activeLayerIndex < 0) return;
      const target = compositePixels[startY]?.[startX];
      if (!target || target.toLowerCase() === selectedColor.toLowerCase()) return;
      const visited = Array.from({ length: data.height }, () =>
        Array.from({ length: data.width }, () => false)
      );
      const queue: Array<[number, number]> = [[startX, startY]];
      setLayers((prev) => {
        const next = prev.map((layer, idx) => {
          if (idx !== activeLayerIndex) return layer;
          const updated = layer.pixels.map((row) => [...row]);
          while (queue.length) {
            const [x, y] = queue.shift() as [number, number];
            if (x < 0 || y < 0 || x >= data.width || y >= data.height) continue;
            if (visited[y][x]) continue;
            visited[y][x] = true;
            const current = compositePixels[y]?.[x];
            if (current?.toLowerCase() !== target.toLowerCase()) continue;
            updated[y][x] = selectedColor;
            queue.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
          }
          return { ...layer, pixels: updated };
        });
        return next;
      });
    },
    [data, activeLayerIndex, compositePixels, selectedColor]
  );

  const getCellFromEvent = useCallback(
    (e: React.PointerEvent) => {
      if (!gridRef.current || !data) return null;
      const rect = gridRef.current.getBoundingClientRect();
      const px = (e.clientX - rect.left) / (CELL_SIZE_PX * zoom);
      const py = (e.clientY - rect.top) / (CELL_SIZE_PX * zoom);
      const x = Math.floor(px);
      const y = Math.floor(py);
      if (x < 0 || y < 0 || x >= data.width || y >= data.height) return null;
      return { x, y };
    },
    [data, zoom]
  );

  const handleGridPointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (e.button === 1 || activeTool === "hand") {
        isPanningRef.current = true;
        pointerStartRef.current = { x: e.clientX, y: e.clientY };
        panStartRef.current = { x: pan.x, y: pan.y };
        return;
      }
      const cell = getCellFromEvent(e);
      if (!cell) return;
      if (activeTool === "fill") {
        floodFill(cell.x, cell.y);
        return;
      }
      isDrawingRef.current = true;
      setPixel(cell.x, cell.y);
    },
    [getCellFromEvent, setPixel, activeTool, pan, floodFill]
  );

  const handleGridPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (isPanningRef.current) {
        const dx = e.clientX - pointerStartRef.current.x;
        const dy = e.clientY - pointerStartRef.current.y;
        setPan({ x: panStartRef.current.x + dx, y: panStartRef.current.y + dy });
        return;
      }
      if (!isDrawingRef.current) return;
      const cell = getCellFromEvent(e);
      if (!cell) return;
      setPixel(cell.x, cell.y);
    },
    [getCellFromEvent, setPixel]
  );

  const handleGridPointerUp = useCallback(() => {
    isDrawingRef.current = false;
    isPanningRef.current = false;
  }, []);

  const handleSaveClick = useCallback(() => {
    if (!data) return;
    setSaveFolderPath(null);
    setSaveError("");
    setSaveFilename("pixel-art.json");
    openFilePicker({
      mode: "folder",
      initialPath: getDefaultInitialPath(),
      onSelect: (folderPath) => {
        setSaveFolderPath(folderPath);
        setSaveModalOpen(true);
      },
    });
  }, [data, openFilePicker]);

  const handleSaveConfirm = useCallback(() => {
    if (!saveFolderPath || !data) return;
    const name = saveFilename.trim();
    if (!name) {
      setSaveError("Enter a file name.");
      return;
    }
    const created = createFile(saveFolderPath, name);
    if (!created) {
      setSaveError("A file with that name already exists.");
      return;
    }
    const path = joinPath(saveFolderPath, name);
    const flattened: PixelArtData = {
      width: data.width,
      height: data.height,
      pixels: compositePixels,
    };
    setFileContent(path, serializePixelArt(flattened));
    setSaveModalOpen(false);
    setSaveError("");
    setSaveSuccess(true);
    setTimeout(() => setSaveSuccess(false), 2000);
  }, [saveFolderPath, saveFilename, data, compositePixels]);

  const handleNewClick = useCallback(() => {
    setData(null);
    setLayers([]);
    setActiveLayerId(null);
  }, []);

  const handleZoom = useCallback((next: number) => {
    const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, next));
    setZoom(clamped);
  }, []);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.5 : 0.5;
      handleZoom(zoom + delta);
    },
    [zoom, handleZoom]
  );

  const addLayer = useCallback(() => {
    if (!data) return;
    const nextIndex = layers.length + 1;
    const newLayer: Layer = {
      id: `layer-${Date.now()}`,
      name: `Layer ${nextIndex}`,
      visible: true,
      pixels: Array.from({ length: data.height }, () => Array.from({ length: data.width }, () => null)),
    };
    setLayers((prev) => [...prev, newLayer]);
    setActiveLayerId(newLayer.id);
  }, [data, layers.length]);

  const removeLayer = useCallback(() => {
    if (layers.length <= 1 || activeLayerId === null) return;
    setLayers((prev) => prev.filter((l) => l.id !== activeLayerId));
    setActiveLayerId((prevId) => {
      const remaining = layers.filter((l) => l.id !== prevId);
      return remaining[remaining.length - 1]?.id ?? null;
    });
  }, [layers, activeLayerId]);

  const toggleLayerVisibility = useCallback((id: string) => {
    setLayers((prev) =>
      prev.map((l) => (l.id === id ? { ...l, visible: !l.visible } : l))
    );
  }, []);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (menuBarRef.current && !menuBarRef.current.contains(e.target as Node)) {
        setFileMenuOpen(false);
        setViewMenuOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.code === "Space" && e.target instanceof HTMLElement && !["INPUT", "TEXTAREA"].includes(e.target.tagName)) {
        e.preventDefault();
        setIsSpacePressed(true);
      }
    }
    function handleKeyUp(e: KeyboardEvent) {
      if (e.code === "Space") setIsSpacePressed(false);
    }
    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("keyup", handleKeyUp);
    };
  }, []);

  useEffect(() => {
    if (!data || !viewportRef.current) return;
    const rect = viewportRef.current.getBoundingClientRect();
    const gridW = data.width * CELL_SIZE_PX * zoom;
    const gridH = data.height * CELL_SIZE_PX * zoom;
    setPan({
      x: Math.round((rect.width - gridW) / 2),
      y: Math.round((rect.height - gridH) / 2),
    });
  }, [data, zoom]);

  if (!data) {
    return (
      <div className={`${styles.app} ${styles.appEmpty}`}>
        <div className={styles.menuBar} ref={menuBarRef}>
          <div className={styles.menuWrap}>
            <button
              type="button"
              className={styles.menuItem}
              onClick={() => setFileMenuOpen((v) => !v)}
            >
              File
            </button>
            {fileMenuOpen && (
              <div className={styles.menuDropdown}>
                {CANVAS_SIZES.map((size) => (
                  <button
                    key={size}
                    type="button"
                    className={styles.menuDropdownItem}
                    onClick={() => startNewArt(size)}
                  >
                    New {size}×{size}
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
        <div className={styles.emptyState}>
          <h2 className={styles.title}>New pixel art</h2>
          <p className={styles.subtitle}>Pick a canvas size to start.</p>
          <div className={styles.newArtGrid}>
            {CANVAS_SIZES.map((size) => (
              <button
                key={size}
                type="button"
                className={styles.newArtCard}
                onClick={() => startNewArt(size)}
              >
                <span className={styles.newArtSize}>{size}×{size}</span>
                <span className={styles.newArtLabel}>New canvas</span>
              </button>
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.app}>
      <div className={styles.menuBar} ref={menuBarRef}>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => setFileMenuOpen((v) => !v)}
          >
            File
          </button>
          {fileMenuOpen && (
            <div className={styles.menuDropdown}>
              {CANVAS_SIZES.map((size) => (
                <button
                  key={size}
                  type="button"
                  className={styles.menuDropdownItem}
                  onClick={() => {
                    startNewArt(size);
                    setFileMenuOpen(false);
                  }}
                >
                  New {size}×{size}
                </button>
              ))}
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => {
                  handleSaveClick();
                  setFileMenuOpen(false);
                }}
              >
                Save
              </button>
            </div>
          )}
        </div>
        <div className={styles.menuWrap}>
          <button
            type="button"
            className={styles.menuItem}
            onClick={() => setViewMenuOpen((v) => !v)}
          >
            View
          </button>
          {viewMenuOpen && (
            <div className={styles.menuDropdown}>
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => handleZoom(zoom + 1)}
              >
                Zoom In
              </button>
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => handleZoom(zoom - 1)}
              >
                Zoom Out
              </button>
              <button
                type="button"
                className={styles.menuDropdownItem}
                onClick={() => handleZoom(1.5)}
              >
                Reset Zoom
              </button>
            </div>
          )}
        </div>
        <div className={styles.menuStatus}>
          {saveSuccess ? "Saved." : `Zoom ${zoom}×`}
        </div>
      </div>
      <div className={styles.layout}>
        <aside className={styles.leftPanel}>
          <div className={styles.panelSection}>
            <span className={styles.panelTitle}>Tools</span>
            <div className={styles.toolList}>
              <button
                type="button"
                className={tool === "pencil" ? styles.toolActive : styles.toolBtn}
                onClick={() => setTool("pencil")}
              >
                <Pencil className={styles.toolIcon} />
                <span>Pencil</span>
              </button>
              <button
                type="button"
                className={tool === "eraser" ? styles.toolActive : styles.toolBtn}
                onClick={() => setTool("eraser")}
              >
                <Eraser className={styles.toolIcon} />
                <span>Eraser</span>
              </button>
              <button
                type="button"
                className={tool === "fill" ? styles.toolActive : styles.toolBtn}
                onClick={() => setTool("fill")}
              >
                <PaintBucket className={styles.toolIcon} />
                <span>Fill</span>
              </button>
              <button
                type="button"
                className={tool === "hand" ? styles.toolActive : styles.toolBtn}
                onClick={() => setTool("hand")}
              >
                <Hand className={styles.toolIcon} />
                <span>Hand</span>
              </button>
            </div>
          </div>
          <div className={styles.panelSection}>
            <span className={styles.panelTitle}>Palette</span>
            <div className={styles.paletteSelectWrap}>
              <select
                className={styles.paletteSelect}
                value={selectedPaletteId}
                onChange={(e) => setSelectedPaletteId(e.target.value)}
                aria-label="Choose palette"
              >
                {PALETTES.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
              <ChevronDown className={styles.paletteSelectArrow} aria-hidden />
            </div>
            <div className={styles.palette}>
              {currentPalette.colors.map((hex) => (
                <button
                  key={hex}
                  type="button"
                  className={styles.swatch}
                  style={{ backgroundColor: hex }}
                  title={hex}
                  aria-pressed={selectedColor === hex}
                  onClick={() => setSelectedColor(hex)}
                />
              ))}
            </div>
            <div className={styles.colorPickerRow}>
              <span className={styles.colorPickerLabel}>Custom</span>
            </div>
            <div className={styles.colorPickerWrap}>
              <HexColorPicker
                color={selectedColor}
                onChange={setSelectedColor}
              />
            </div>
          </div>
        </aside>
        <section className={styles.viewportPanel}>
          <div className={styles.viewport} onWheel={handleWheel} ref={viewportRef}>
            <div
              className={styles.panLayer}
              style={{ transform: `translate(${pan.x}px, ${pan.y}px)` }}
            >
              <div
                ref={gridRef}
                className={activeTool === "hand" ? styles.gridWrapHand : styles.gridWrap}
                style={{
                  width: data.width * CELL_SIZE_PX,
                  height: data.height * CELL_SIZE_PX,
                  gridTemplateColumns: `repeat(${data.width}, ${CELL_SIZE_PX}px)`,
                  gridTemplateRows: `repeat(${data.height}, ${CELL_SIZE_PX}px)`,
                  transform: `scale(${zoom})`,
                  transformOrigin: "top left",
                }}
                onPointerDown={handleGridPointerDown}
                onPointerMove={handleGridPointerMove}
                onPointerUp={handleGridPointerUp}
                onPointerLeave={handleGridPointerUp}
              >
                {compositePixels.map((row, y) =>
                  row.map((color, x) => (
                    <div
                      key={`${y}-${x}`}
                      className={styles.cell}
                      data-x={x}
                      data-y={y}
                      style={{ backgroundColor: color }}
                    />
                  ))
                )}
              </div>
            </div>
          </div>
          <div className={styles.zoomRow}>
            <span className={styles.zoomLabel}>Zoom</span>
            <input
              className={styles.zoomSlider}
              type="range"
              min={MIN_ZOOM}
              max={MAX_ZOOM}
              step={1}
              value={zoom}
              onChange={(e) => handleZoom(parseInt(e.target.value, 10))}
            />
            <span className={styles.zoomValue}>{zoom}×</span>
          </div>
        </section>
        <aside className={styles.rightPanel}>
          <div className={styles.panelSection}>
            <div className={styles.previewHeader}>
              <span className={styles.panelTitle}>Preview</span>
              <div className={styles.previewControls}>
                <button
                  type="button"
                  className={styles.previewBtn}
                  onClick={() => setPreviewZoom((z) => Math.max(1, z - 1))}
                >
                  –
                </button>
                <span className={styles.previewZoom}>{previewZoom}×</span>
                <button
                  type="button"
                  className={styles.previewBtn}
                  onClick={() => setPreviewZoom((z) => Math.min(2, z + 1))}
                >
                  +
                </button>
              </div>
            </div>
            <div className={styles.previewBox}>
              <div
                className={styles.previewImageWrap}
                style={{
                  transform: `scale(${previewZoom * 3})`,
                  transformOrigin: "center",
                }}
              >
                {previewUrl ? (
                  <img
                    src={previewUrl}
                    width={data.width}
                    height={data.height}
                    className={styles.previewImage}
                    alt="Pixel art preview"
                  />
                ) : (
                  <div className={styles.previewPlaceholder} />
                )}
              </div>
            </div>
          </div>
          <div className={styles.panelSection}>
            <div className={styles.layerHeader}>
              <span className={styles.panelTitle}>Layers</span>
              <div className={styles.layerActions}>
                <button type="button" className={styles.layerBtn} onClick={addLayer}>
                  Add
                </button>
                <button
                  type="button"
                  className={styles.layerBtn}
                  onClick={removeLayer}
                  disabled={layers.length <= 1}
                >
                  Delete
                </button>
              </div>
            </div>
            <div className={styles.layerList}>
              {layers.map((layer) => (
                <button
                  key={layer.id}
                  type="button"
                  className={layer.id === activeLayerId ? styles.layerItemActive : styles.layerItem}
                  onClick={() => setActiveLayerId(layer.id)}
                >
                  <button
                    type="button"
                    className={layer.visible ? styles.layerEyeOn : styles.layerEyeOff}
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleLayerVisibility(layer.id);
                    }}
                    aria-label={layer.visible ? "Hide layer" : "Show layer"}
                    title={layer.visible ? "Hide layer" : "Show layer"}
                  >
                    {layer.visible ? "On" : "Off"}
                  </button>
                  <span className={styles.layerName}>{layer.name}</span>
                </button>
              ))}
            </div>
          </div>
        </aside>
      </div>
      <Modal
        open={saveModalOpen}
        onClose={() => setSaveModalOpen(false)}
        title="Save pixel art"
        primaryButton={{ label: "Save", onClick: handleSaveConfirm }}
        secondaryButton={{ label: "Cancel", onClick: () => setSaveModalOpen(false) }}
      >
        <div className={styles.saveModalContent}>
          <label className={styles.saveLabel}>
            File name
            <input
              type="text"
              value={saveFilename}
              onChange={(e) => setSaveFilename(e.target.value)}
              className={styles.saveInput}
              placeholder="pixel-art.json"
            />
          </label>
          {saveError && <p className={styles.saveError}>{saveError}</p>}
        </div>
      </Modal>
    </div>
  );
}

