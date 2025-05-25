import React, { useCallback, useState, useRef } from 'react';
import { cn } from '@/lib/utils';
import {
  IconPaperclip,
  IconUpload,
  IconX,
  IconFile,
  IconLoader2
} from '@tabler/icons-react';
import { 
  isValidFileType, 
  getMaxFileSize, 
  formatFileSize, 
  processFileUpload 
} from '@/services/fileService';
import { FileUploadState, ProcessedFile } from '@/types/files';
import { open } from '@tauri-apps/plugin-dialog';

interface FileUploadProps {
  onFileProcessed: (processedFile: ProcessedFile) => void;
  onFileRemoved: () => void;
  className?: string;
  disabled?: boolean;
}

interface FileWithPath extends File {
  path?: string;
}

const FileUpload: React.FC<FileUploadProps> = ({
  onFileProcessed,
  onFileRemoved,
  className,
  disabled = false,
}) => {
  console.log('=== FileUpload component rendered ===');
  console.log('Props:', { disabled, className });
  
  const [uploadState, setUploadState] = useState<FileUploadState>({
    isProcessing: false,
  });
  const [isDragOver, setIsDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  
  // Debug: Log when component mounts
  React.useEffect(() => {
    console.log('FileUpload component mounted');
    return () => {
      console.log('FileUpload component unmounted');
    };
  }, []);

  const validateFile = useCallback((file: File): string | null => {
    if (!isValidFileType(file)) {
      return 'Unsupported file type. Supported: .txt, .md, .json, .csv, .pdf, .docx';
    }

    // Skip size validation for mock files (size = 0) as we'll validate on the backend
    if (file.size > 0) {
      const maxSize = getMaxFileSize(file);
      if (file.size > maxSize) {
        return `File too large. Maximum size: ${formatFileSize(maxSize)}`;
      }
    }

    return null;
  }, []);

  const handleFileSelect = useCallback(async (file: FileWithPath) => {
    console.log('=== FILE SELECTION STARTED ===');
    console.log('File object:', {
      name: file.name,
      size: file.size,
      type: file.type,
      path: file.path,
      lastModified: file.lastModified
    });

    const validationError = validateFile(file);
    if (validationError) {
      console.error('File validation failed:', validationError);
      setUploadState({
        selectedFile: undefined,
        isProcessing: false,
        error: validationError,
      });
      return;
    }

    console.log('File validation passed');

    setUploadState({
      selectedFile: file,
      isProcessing: true,
      error: undefined,
    });

    try {
      // For Tauri, we need to get the file path
      const filePath = file.path;
      console.log('File path for processing:', filePath);
      
      if (!filePath) {
        const error = 'File path not available. Please select a file from the file system.';
        console.error(error);
        throw new Error(error);
      }

      console.log('Starting file processing...');
      const processedFile = await processFileUpload(filePath);
      console.log('=== FILE PROCESSING SUCCESSFUL ===');
      console.log('Processed file:', {
        contentLength: processedFile.content.length,
        metadata: processedFile.metadata,
        processingTime: processedFile.processing_time
      });
      
      setUploadState({
        selectedFile: file,
        isProcessing: false,
        processedContent: processedFile,
        error: undefined,
      });

      onFileProcessed(processedFile);
    } catch (error) {
      console.error('=== FILE PROCESSING FAILED ===');
      console.error('Error details:', error);
      console.error('Error type:', typeof error);
      console.error('Error message:', error instanceof Error ? error.message : String(error));
      
      const errorMessage = error instanceof Error ? error.message : 'Failed to process file';
      console.error('Final error message:', errorMessage);
      
      setUploadState({
        selectedFile: file,
        isProcessing: false,
        error: errorMessage,
      });
    }
  }, [validateFile, onFileProcessed]);


  const handleFilePicker = useCallback(async () => {
    console.log('=== FILE PICKER STARTED ===');
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Supported Files',
            extensions: ['txt', 'md', 'json', 'csv', 'pdf', 'docx'],
          },
        ],
      });

      console.log('File picker result:', selected);
      console.log('File picker result type:', typeof selected);
      console.log('File picker result value:', JSON.stringify(selected));

      // Handle different possible return types from Tauri dialog
      let filePath: string | null = null;
      
      if (selected === null || selected === undefined) {
        console.log('File picker was cancelled by user');
        return;
      }
      
      if (typeof selected === 'string') {
        filePath = selected;
      } else if (Array.isArray(selected) && selected.length > 0) {
        // Handle case where dialog returns an array even with multiple: false
        filePath = selected[0];
      } else if (selected && typeof selected === 'object' && 'path' in selected) {
        // Handle case where dialog returns an object with path property
        filePath = (selected as Record<string, unknown>).path as string;
      }

      if (!filePath) {
        console.log('No valid file path found in dialog result');
        setUploadState(prev => ({
          ...prev,
          error: 'No file was selected or file path could not be determined',
        }));
        return;
      }

      console.log('Selected file path:', filePath);
      
      // Create a mock File object with the path
      const fileName = filePath.split(/[/\\]/).pop() || 'unknown'; // Handle both Unix and Windows paths
      console.log('Extracted file name:', fileName);
      
      const mockFile: FileWithPath = {
        name: fileName,
        size: 0, // We'll get the real size from the backend
        type: '',
        path: filePath,
        lastModified: Date.now(),
        webkitRelativePath: '',
        arrayBuffer: async () => new ArrayBuffer(0),
        slice: () => new Blob(),
        stream: () => new ReadableStream(),
        text: async () => '',
        bytes: async () => new Uint8Array(0),
      };

      console.log('Created mock file object:', mockFile);
      await handleFileSelect(mockFile);
    } catch (error) {
      console.error('=== FILE PICKER FAILED ===');
      console.error('Error opening file picker:', error);
      console.error('Error type:', typeof error);
      console.error('Error details:', JSON.stringify(error));
      setUploadState(prev => ({
        ...prev,
        error: `Failed to open file picker: ${error instanceof Error ? error.message : String(error)}`,
      }));
    }
  }, [handleFileSelect]);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!disabled && !uploadState.isProcessing) {
      setIsDragOver(true);
    }
  }, [disabled, uploadState.isProcessing]);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragOver(false);

    console.log('=== FILE DROP STARTED ===');

    if (disabled || uploadState.isProcessing) {
      console.log('Drop ignored - disabled or processing');
      return;
    }

    const files = Array.from(e.dataTransfer.files);
    console.log('Dropped files:', files.map(f => ({ name: f.name, size: f.size, type: f.type })));
    
    if (files.length > 0) {
      const file = files[0] as FileWithPath;
      console.log('Processing first dropped file:', file.name);
      
      // For dropped files in Tauri, we don't have the file path
      // We need to show an error and suggest using the file picker instead
      console.warn('Drag and drop not fully supported in Tauri - file path not available');
      setUploadState(prev => ({
        ...prev,
        error: 'Drag and drop is not supported. Please use the file picker button to select files.',
      }));
    }
  }, [disabled, uploadState.isProcessing]);

  const handleFileInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      const file = files[0] as FileWithPath;
      handleFileSelect(file);
    }
  }, [handleFileSelect]);

  const handleRemoveFile = useCallback(() => {
    setUploadState({
      isProcessing: false,
    });
    onFileRemoved();
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  }, [onFileRemoved]);

  // If we have a selected file, show the file info
  if (uploadState.selectedFile) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <div className="flex items-center gap-2 px-2 py-1 bg-main-view-fg/5 rounded-md">
          <IconFile size={16} className="text-main-view-fg/70" />
          <div className="flex flex-col">
            <span className="text-xs font-medium text-main-view-fg">
              {uploadState.selectedFile.name}
            </span>
            {uploadState.selectedFile.size > 0 && (
              <span className="text-xs text-main-view-fg/60">
                {formatFileSize(uploadState.selectedFile.size)}
              </span>
            )}
          </div>
          {uploadState.isProcessing && (
            <IconLoader2 size={16} className="animate-spin text-main-view-fg/70" />
          )}
          {!uploadState.isProcessing && (
            <button
              className="h-4 w-4 flex items-center justify-center hover:bg-main-view-fg/10 rounded"
              onClick={handleRemoveFile}
            >
              <IconX size={12} />
            </button>
          )}
        </div>
        {uploadState.error && (
          <span className="text-xs text-destructive">{uploadState.error}</span>
        )}
      </div>
    );
  }

  // Show the upload interface
  return (
    <div className={cn('relative', className)}>
      <div
        className={cn(
          'h-6 p-1 flex items-center justify-center rounded-sm hover:bg-main-view-fg/10 transition-all duration-200 ease-in-out cursor-pointer',
          isDragOver && 'bg-primary/10 border-2 border-dashed border-primary',
          (disabled || uploadState.isProcessing) && 'opacity-50 cursor-not-allowed'
        )}
        onClick={!disabled && !uploadState.isProcessing ? handleFilePicker : undefined}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
      >
        {uploadState.isProcessing ? (
          <IconLoader2 size={18} className="animate-spin text-main-view-fg/50" />
        ) : isDragOver ? (
          <IconUpload size={18} className="text-primary" />
        ) : (
          <IconPaperclip size={18} className="text-main-view-fg/50" />
        )}
      </div>

      {/* Hidden file input for fallback */}
      <input
        ref={fileInputRef}
        type="file"
        accept=".txt,.md,.json,.csv,.pdf,.docx"
        onChange={handleFileInputChange}
        className="hidden"
        disabled={disabled || uploadState.isProcessing}
      />

      {uploadState.error && (
        <div className="absolute top-full left-0 mt-1 text-xs text-destructive whitespace-nowrap z-10">
          {uploadState.error}
        </div>
      )}
    </div>
  );
};

export default FileUpload;