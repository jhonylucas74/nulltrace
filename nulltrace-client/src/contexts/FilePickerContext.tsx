import React, { createContext, useContext, useState, useCallback } from "react";
import { getHomePath } from "../lib/fileSystem";

export type FilePickerMode = "folder" | "file";

export interface FilePickerOptions {
  mode: FilePickerMode;
  /** Initial path to show (folder path). Defaults to home. */
  initialPath?: string;
  onSelect: (path: string) => void;
  onCancel?: () => void;
}

interface FilePickerContextValue {
  isOpen: boolean;
  openFilePicker: (options: FilePickerOptions) => void;
  closeFilePicker: () => void;
  /** Current options when open (for the modal to read). */
  options: FilePickerOptions | null;
}

const FilePickerContext = createContext<FilePickerContextValue | null>(null);

export function FilePickerProvider({ children }: { children: React.ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [options, setOptions] = useState<FilePickerOptions | null>(null);

  const openFilePicker = useCallback((opts: FilePickerOptions) => {
    setOptions(opts);
    setIsOpen(true);
  }, []);

  const closeFilePicker = useCallback(() => {
    setIsOpen(false);
    setOptions(null);
  }, []);

  const value: FilePickerContextValue = {
    isOpen,
    openFilePicker,
    closeFilePicker,
    options,
  };

  return (
    <FilePickerContext.Provider value={value}>
      {children}
    </FilePickerContext.Provider>
  );
}

export function useFilePicker(): FilePickerContextValue {
  const ctx = useContext(FilePickerContext);
  if (!ctx) throw new Error("useFilePicker must be used within FilePickerProvider");
  return ctx;
}

export function getDefaultInitialPath(): string {
  return getHomePath();
}
