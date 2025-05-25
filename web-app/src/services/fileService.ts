import { invoke } from '@tauri-apps/api/core';
import { ProcessedFile } from '@/types/files';

/**
 * Process a file upload using the Tauri backend
 * @param filePath - The absolute path to the file to process
 * @returns Promise<ProcessedFile> - The processed file with content and metadata
 */
export async function processFileUpload(filePath: string): Promise<ProcessedFile> {
  console.log('=== FILE SERVICE: PROCESSING STARTED ===');
  console.log('File path:', filePath);
  console.log('File path type:', typeof filePath);
  console.log('File path length:', filePath.length);
  console.log('Invoking Tauri command: process_file_upload');
  
  // Validate file path before sending to backend
  if (!filePath || typeof filePath !== 'string' || filePath.trim().length === 0) {
    throw new Error('Invalid file path provided');
  }
  
  try {
    const startTime = Date.now();
    console.log('Calling invoke with parameters:', { file_path: filePath });
    
    const result = await invoke<ProcessedFile>('process_file_upload', {
      filePath: filePath,
    });
    
    const endTime = Date.now();
    
    console.log('=== FILE SERVICE: PROCESSING SUCCESSFUL ===');
    console.log('Processing time (frontend):', endTime - startTime, 'ms');
    console.log('Result:', {
      contentLength: result.content.length,
      metadata: result.metadata,
      processingTime: result.processing_time
    });
    
    // Validate the result structure
    if (!result || typeof result !== 'object') {
      throw new Error('Invalid response from backend: result is not an object');
    }
    
    if (!result.content || typeof result.content !== 'string') {
      throw new Error('Invalid response from backend: missing or invalid content');
    }
    
    if (!result.metadata || typeof result.metadata !== 'object') {
      throw new Error('Invalid response from backend: missing or invalid metadata');
    }
    
    return result;
  } catch (error) {
    console.error('=== FILE SERVICE: PROCESSING FAILED ===');
    console.error('File path:', filePath);
    console.error('Error object:', error);
    console.error('Error type:', typeof error);
    console.error('Error constructor:', error?.constructor?.name);
    
    if (error && typeof error === 'object') {
      console.error('Error properties:', Object.keys(error));
      console.error('Error message:', (error as Record<string, unknown>).message);
      console.error('Error toString:', error.toString());
    }
    
    let errorMessage = 'Failed to process file';
    
    if (error instanceof Error) {
      errorMessage = error.message;
    } else if (error && typeof error === 'object' && 'message' in error) {
      errorMessage = String((error as Record<string, unknown>).message);
    } else if (typeof error === 'string') {
      errorMessage = error;
    }
    
    console.error('Final error message:', errorMessage);
    
    throw new Error(errorMessage);
  }
}

/**
 * Validate file type on the client side
 * @param file - The File object to validate
 * @returns boolean - Whether the file type is supported
 */
export function isValidFileType(file: File): boolean {
  const supportedTypes = [
    // Text files
    'text/plain',
    'text/markdown',
    'application/json',
    // CSV files
    'text/csv',
    'application/csv',
    // PDF files
    'application/pdf',
    // DOCX files
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  ];

  const supportedExtensions = ['.txt', '.md', '.json', '.csv', '.pdf', '.docx'];
  
  const fileExtension = '.' + file.name.split('.').pop()?.toLowerCase();
  
  return (
    supportedTypes.includes(file.type) ||
    supportedExtensions.includes(fileExtension)
  );
}

/**
 * Get the maximum file size for a given file type
 * @param file - The File object to check
 * @returns number - Maximum file size in bytes
 */
export function getMaxFileSize(file: File): number {
  const fileExtension = '.' + file.name.split('.').pop()?.toLowerCase();
  
  switch (fileExtension) {
    case '.txt':
    case '.md':
    case '.json':
      return 5 * 1024 * 1024; // 5MB
    case '.csv':
      return 10 * 1024 * 1024; // 10MB
    case '.pdf':
      return 20 * 1024 * 1024; // 20MB
    case '.docx':
      return 15 * 1024 * 1024; // 15MB
    default:
      return 5 * 1024 * 1024; // Default 5MB
  }
}

/**
 * Format file size for display
 * @param bytes - File size in bytes
 * @returns string - Formatted file size
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}