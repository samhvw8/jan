interface FileMetadata {
  name: string;
  size: number;
  file_type: string;
  encoding?: string;
}

interface ProcessedFile {
  content: string;
  metadata: FileMetadata;
  processing_time: number;
}

interface FileUploadState {
  selectedFile?: File;
  isProcessing: boolean;
  processedContent?: ProcessedFile;
  error?: string;
}

export type { FileMetadata, ProcessedFile, FileUploadState };