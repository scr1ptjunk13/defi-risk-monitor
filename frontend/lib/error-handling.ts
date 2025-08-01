import { toast } from 'react-hot-toast';

// Enhanced Error Types
export enum ErrorSeverity {
  LOW = 'low',
  MEDIUM = 'medium',
  HIGH = 'high',
  CRITICAL = 'critical'
}

export enum ErrorCategory {
  NETWORK = 'network',
  VALIDATION = 'validation',
  AUTHENTICATION = 'authentication',
  AUTHORIZATION = 'authorization',
  BLOCKCHAIN = 'blockchain',
  API = 'api',
  UI = 'ui',
  UNKNOWN = 'unknown'
}

export interface ErrorContext {
  userId?: string;
  action?: string;
  component?: string;
  timestamp: Date;
  userAgent: string;
  url: string;
  additionalData?: Record<string, any>;
}

export interface EnhancedError {
  id: string;
  message: string;
  userMessage: string;
  severity: ErrorSeverity;
  category: ErrorCategory;
  context: ErrorContext;
  originalError?: Error;
  stack?: string;
  recoverable: boolean;
  retryable: boolean;
  suggestions?: string[];
}

// Error Handler Class
export class ErrorHandler {
  private static instance: ErrorHandler;
  private errorLog: EnhancedError[] = [];
  private maxLogSize = 100;

  private constructor() {}

  static getInstance(): ErrorHandler {
    if (!ErrorHandler.instance) {
      ErrorHandler.instance = new ErrorHandler();
    }
    return ErrorHandler.instance;
  }

  // Main error handling method
  handleError(
    error: Error | string,
    options: {
      severity?: ErrorSeverity;
      category?: ErrorCategory;
      component?: string;
      action?: string;
      showToast?: boolean;
      logToConsole?: boolean;
      additionalData?: Record<string, any>;
    } = {}
  ): EnhancedError {
    const {
      severity = ErrorSeverity.MEDIUM,
      category = ErrorCategory.UNKNOWN,
      component = 'Unknown',
      action = 'Unknown',
      showToast = true,
      logToConsole = true,
      additionalData = {}
    } = options;

    const enhancedError = this.createEnhancedError(
      error,
      severity,
      category,
      component,
      action,
      additionalData
    );

    // Log to internal storage
    this.logError(enhancedError);

    // Console logging for developers
    if (logToConsole) {
      this.logToConsole(enhancedError);
    }

    // User-facing notifications
    if (showToast) {
      this.showUserNotification(enhancedError);
    }

    // Send to monitoring service (if configured)
    this.sendToMonitoring(enhancedError);

    return enhancedError;
  }

  private createEnhancedError(
    error: Error | string,
    severity: ErrorSeverity,
    category: ErrorCategory,
    component: string,
    action: string,
    additionalData: Record<string, any>
  ): EnhancedError {
    const errorMessage = typeof error === 'string' ? error : error.message;
    const originalError = typeof error === 'string' ? undefined : error;

    return {
      id: this.generateErrorId(),
      message: errorMessage,
      userMessage: this.generateUserMessage(errorMessage, category),
      severity,
      category,
      context: {
        component,
        action,
        timestamp: new Date(),
        userAgent: navigator.userAgent,
        url: window.location.href,
        additionalData
      },
      originalError,
      stack: originalError?.stack,
      recoverable: this.isRecoverable(category, severity),
      retryable: this.isRetryable(category, errorMessage),
      suggestions: this.generateSuggestions(category, errorMessage)
    };
  }

  private generateErrorId(): string {
    return `err_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  private generateUserMessage(errorMessage: string, category: ErrorCategory): string {
    // Convert technical errors to user-friendly messages
    const userMessages: Record<ErrorCategory, (msg: string) => string> = {
      [ErrorCategory.NETWORK]: (msg) => {
        if (msg.includes('timeout')) return 'Connection timed out. Please check your internet connection.';
        if (msg.includes('offline')) return 'You appear to be offline. Please check your connection.';
        return 'Network error occurred. Please try again.';
      },
      [ErrorCategory.VALIDATION]: (msg) => {
        if (msg.includes('required')) return 'Please fill in all required fields.';
        if (msg.includes('invalid')) return 'Please check your input and try again.';
        return 'Input validation failed. Please review your entries.';
      },
      [ErrorCategory.AUTHENTICATION]: () => 'Please sign in to continue.',
      [ErrorCategory.AUTHORIZATION]: () => 'You don\'t have permission to perform this action.',
      [ErrorCategory.BLOCKCHAIN]: (msg) => {
        if (msg.includes('rejected')) return 'Transaction was rejected. Please try again.';
        if (msg.includes('insufficient')) return 'Insufficient funds for this transaction.';
        if (msg.includes('gas')) return 'Transaction failed due to gas issues. Please adjust gas settings.';
        return 'Blockchain transaction failed. Please try again.';
      },
      [ErrorCategory.API]: (msg) => {
        if (msg.includes('404')) return 'The requested resource was not found.';
        if (msg.includes('500')) return 'Server error occurred. Please try again later.';
        if (msg.includes('429')) return 'Too many requests. Please wait a moment and try again.';
        return 'Service temporarily unavailable. Please try again.';
      },
      [ErrorCategory.UI]: () => 'Interface error occurred. Please refresh the page.',
      [ErrorCategory.UNKNOWN]: () => 'An unexpected error occurred. Please try again.'
    };

    return userMessages[category](errorMessage);
  }

  private isRecoverable(category: ErrorCategory, severity: ErrorSeverity): boolean {
    if (severity === ErrorSeverity.CRITICAL) return false;
    return [
      ErrorCategory.NETWORK,
      ErrorCategory.VALIDATION,
      ErrorCategory.API,
      ErrorCategory.UI
    ].includes(category);
  }

  private isRetryable(category: ErrorCategory, errorMessage: string): boolean {
    if (category === ErrorCategory.VALIDATION) return false;
    if (errorMessage.includes('429') || errorMessage.includes('timeout')) return true;
    return [ErrorCategory.NETWORK, ErrorCategory.API].includes(category);
  }

  private generateSuggestions(category: ErrorCategory, errorMessage: string): string[] {
    const suggestions: Record<ErrorCategory, string[]> = {
      [ErrorCategory.NETWORK]: [
        'Check your internet connection',
        'Try refreshing the page',
        'Disable VPN if enabled'
      ],
      [ErrorCategory.VALIDATION]: [
        'Review all form fields',
        'Check for required fields',
        'Ensure proper format for inputs'
      ],
      [ErrorCategory.AUTHENTICATION]: [
        'Sign in to your account',
        'Clear browser cache and cookies',
        'Try incognito/private browsing mode'
      ],
      [ErrorCategory.AUTHORIZATION]: [
        'Contact support for access',
        'Verify your account permissions',
        'Try signing out and back in'
      ],
      [ErrorCategory.BLOCKCHAIN]: [
        'Check your wallet connection',
        'Ensure sufficient gas fees',
        'Try increasing gas limit',
        'Verify network selection'
      ],
      [ErrorCategory.API]: [
        'Wait a moment and try again',
        'Check service status',
        'Contact support if issue persists'
      ],
      [ErrorCategory.UI]: [
        'Refresh the page',
        'Clear browser cache',
        'Try a different browser'
      ],
      [ErrorCategory.UNKNOWN]: [
        'Try again in a few moments',
        'Refresh the page',
        'Contact support if issue persists'
      ]
    };

    return suggestions[category] || suggestions[ErrorCategory.UNKNOWN];
  }

  private logError(error: EnhancedError): void {
    this.errorLog.unshift(error);
    if (this.errorLog.length > this.maxLogSize) {
      this.errorLog = this.errorLog.slice(0, this.maxLogSize);
    }
  }

  private logToConsole(error: EnhancedError): void {
    const logMethod = {
      [ErrorSeverity.LOW]: console.info,
      [ErrorSeverity.MEDIUM]: console.warn,
      [ErrorSeverity.HIGH]: console.error,
      [ErrorSeverity.CRITICAL]: console.error
    }[error.severity];

    logMethod(
      `🚨 ${error.severity.toUpperCase()} ERROR [${error.category}] in ${error.context.component}:`,
      {
        id: error.id,
        message: error.message,
        userMessage: error.userMessage,
        context: error.context,
        suggestions: error.suggestions,
        stack: error.stack
      }
    );
  }

  private showUserNotification(error: EnhancedError): void {
    const toastOptions = {
      duration: this.getToastDuration(error.severity),
      position: 'top-right' as const,
    };

    switch (error.severity) {
      case ErrorSeverity.LOW:
        toast(error.userMessage, { ...toastOptions, icon: 'ℹ️' });
        break;
      case ErrorSeverity.MEDIUM:
        toast(error.userMessage, { ...toastOptions, icon: '⚠️' });
        break;
      case ErrorSeverity.HIGH:
        toast.error(error.userMessage, toastOptions);
        break;
      case ErrorSeverity.CRITICAL:
        toast.error(`Critical Error: ${error.userMessage}`, {
          ...toastOptions,
          duration: 8000
        });
        break;
    }
  }

  private getToastDuration(severity: ErrorSeverity): number {
    return {
      [ErrorSeverity.LOW]: 3000,
      [ErrorSeverity.MEDIUM]: 4000,
      [ErrorSeverity.HIGH]: 6000,
      [ErrorSeverity.CRITICAL]: 8000
    }[severity];
  }

  private sendToMonitoring(error: EnhancedError): void {
    // In a real application, send to monitoring service like Sentry, LogRocket, etc.
    if (process.env.NODE_ENV === 'production') {
      // Example: Sentry.captureException(error);
      console.info('Error would be sent to monitoring service:', error.id);
    }
  }

  // Public methods for error management
  getRecentErrors(limit: number = 10): EnhancedError[] {
    return this.errorLog.slice(0, limit);
  }

  getErrorById(id: string): EnhancedError | undefined {
    return this.errorLog.find(error => error.id === id);
  }

  clearErrorLog(): void {
    this.errorLog = [];
  }

  getErrorStats(): {
    total: number;
    bySeverity: Record<ErrorSeverity, number>;
    byCategory: Record<ErrorCategory, number>;
  } {
    const stats = {
      total: this.errorLog.length,
      bySeverity: {} as Record<ErrorSeverity, number>,
      byCategory: {} as Record<ErrorCategory, number>
    };

    // Initialize counters
    Object.values(ErrorSeverity).forEach(severity => {
      stats.bySeverity[severity] = 0;
    });
    Object.values(ErrorCategory).forEach(category => {
      stats.byCategory[category] = 0;
    });

    // Count errors
    this.errorLog.forEach(error => {
      stats.bySeverity[error.severity]++;
      stats.byCategory[error.category]++;
    });

    return stats;
  }
}

// Convenience functions for common error scenarios
export const errorHandler = ErrorHandler.getInstance();

export const handleNetworkError = (error: Error, component?: string, action?: string) =>
  errorHandler.handleError(error, {
    severity: ErrorSeverity.HIGH,
    category: ErrorCategory.NETWORK,
    component,
    action
  });

export const handleValidationError = (message: string, component?: string) =>
  errorHandler.handleError(message, {
    severity: ErrorSeverity.MEDIUM,
    category: ErrorCategory.VALIDATION,
    component,
    showToast: true
  });

export const handleBlockchainError = (error: Error, component?: string, action?: string) =>
  errorHandler.handleError(error, {
    severity: ErrorSeverity.HIGH,
    category: ErrorCategory.BLOCKCHAIN,
    component,
    action
  });

export const handleAPIError = (error: Error, component?: string, action?: string) =>
  errorHandler.handleError(error, {
    severity: ErrorSeverity.MEDIUM,
    category: ErrorCategory.API,
    component,
    action
  });

export const handleCriticalError = (error: Error, component?: string, action?: string) =>
  errorHandler.handleError(error, {
    severity: ErrorSeverity.CRITICAL,
    category: ErrorCategory.UNKNOWN,
    component,
    action
  });

// React Hook for error handling
export const useErrorHandler = () => {
  return {
    handleError: errorHandler.handleError.bind(errorHandler),
    handleNetworkError,
    handleValidationError,
    handleBlockchainError,
    handleAPIError,
    handleCriticalError,
    getRecentErrors: errorHandler.getRecentErrors.bind(errorHandler),
    getErrorStats: errorHandler.getErrorStats.bind(errorHandler),
    clearErrorLog: errorHandler.clearErrorLog.bind(errorHandler)
  };
};
