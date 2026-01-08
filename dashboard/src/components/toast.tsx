'use client';

import {
  createContext,
  useContext,
  useState,
  useCallback,
  useRef,
  useEffect,
  ReactNode,
} from 'react';
import { AlertCircle, CheckCircle2, Info, X, Copy, Check } from 'lucide-react';
import { cn } from '@/lib/utils';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

type ToastType = 'success' | 'error' | 'info';

interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message: string;
  duration: number;
}

interface ToastContextValue {
  showSuccess: (message: string, title?: string) => void;
  showError: (message: string, title?: string) => void;
  showInfo: (message: string, title?: string) => void;
}

// ─────────────────────────────────────────────────────────────────────────────
// Toast styling config
// ─────────────────────────────────────────────────────────────────────────────

const toastStyles: Record<
  ToastType,
  { border: string; text: string; progress: string; defaultTitle: string }
> = {
  success: {
    border: 'border-emerald-500/30',
    text: 'text-emerald-400',
    progress: 'bg-emerald-500/50',
    defaultTitle: 'Success',
  },
  error: {
    border: 'border-red-500/30',
    text: 'text-red-400',
    progress: 'bg-red-500/50',
    defaultTitle: 'Error',
  },
  info: {
    border: 'border-indigo-500/30',
    text: 'text-indigo-400',
    progress: 'bg-indigo-500/50',
    defaultTitle: 'Info',
  },
};

const toastIcons: Record<ToastType, typeof CheckCircle2> = {
  success: CheckCircle2,
  error: AlertCircle,
  info: Info,
};

const toastDurations: Record<ToastType, number> = {
  success: 4000,
  error: 8000,
  info: 4000,
};

// ─────────────────────────────────────────────────────────────────────────────
// Global toast function (for compatibility with sonner API)
// ─────────────────────────────────────────────────────────────────────────────

let globalAddToast: ((type: ToastType, message: string, title?: string) => void) | null =
  null;

export const toast = {
  success: (message: string) => {
    if (globalAddToast) {
      globalAddToast('success', message);
    } else {
      console.warn('Toast provider not initialized');
    }
  },
  error: (message: string) => {
    if (globalAddToast) {
      globalAddToast('error', message);
    } else {
      console.warn('Toast provider not initialized');
    }
  },
  info: (message: string) => {
    if (globalAddToast) {
      globalAddToast('info', message);
    } else {
      console.warn('Toast provider not initialized');
    }
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Context
// ─────────────────────────────────────────────────────────────────────────────

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within ToastProvider');
  }
  return context;
}

// Alias for backwards compatibility with ErrorToastProvider
export const useErrorToast = () => {
  const { showError } = useToast();
  return { showError };
};

// ─────────────────────────────────────────────────────────────────────────────
// Toast Item Component
// ─────────────────────────────────────────────────────────────────────────────

interface ToastItemProps {
  toast: Toast;
  onDismiss: (id: string) => void;
  onShowDetails: (message: string) => void;
}

function ToastItem({ toast, onDismiss, onShowDetails }: ToastItemProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const [progress, setProgress] = useState(100);
  const startTimeRef = useRef<number>(Date.now());
  const remainingRef = useRef<number>(toast.duration);
  const frameRef = useRef<number | null>(null);

  const dismiss = useCallback(() => {
    if (isExiting) return;
    setIsExiting(true);
    setTimeout(() => onDismiss(toast.id), 200);
  }, [onDismiss, toast.id, isExiting]);

  useEffect(() => {
    const animate = () => {
      if (isHovered || isExiting) {
        frameRef.current = requestAnimationFrame(animate);
        return;
      }

      const elapsed = Date.now() - startTimeRef.current;
      const remaining = remainingRef.current - elapsed;

      if (remaining <= 0) {
        dismiss();
        return;
      }

      setProgress((remaining / toast.duration) * 100);
      frameRef.current = requestAnimationFrame(animate);
    };

    frameRef.current = requestAnimationFrame(animate);

    return () => {
      if (frameRef.current) {
        cancelAnimationFrame(frameRef.current);
      }
    };
  }, [isHovered, isExiting, toast.duration, dismiss]);

  // Pause/resume timer on hover
  useEffect(() => {
    if (isHovered) {
      // Pausing: save remaining time
      remainingRef.current = remainingRef.current - (Date.now() - startTimeRef.current);
    } else {
      // Resuming: reset start time
      startTimeRef.current = Date.now();
    }
  }, [isHovered]);

  const style = toastStyles[toast.type];
  const Icon = toastIcons[toast.type];
  const truncated =
    toast.message.length > 100 ? toast.message.slice(0, 100) + '...' : toast.message;
  const hasDetails = toast.message.length > 100;

  const handleClick = () => {
    if (hasDetails) {
      onShowDetails(toast.message);
      dismiss();
    }
  };

  return (
    <div
      className={cn(
        'relative flex items-start gap-3 p-4 rounded-xl border shadow-lg transition-all duration-200 max-w-[400px] overflow-hidden bg-[#1c1c1e]/95',
        style.border,
        hasDetails && 'cursor-pointer hover:bg-[#2c2c2e]/95',
        isExiting ? 'animate-toast-out' : 'animate-toast-in'
      )}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={handleClick}
    >
      {/* Icon */}
      <Icon className={cn('h-5 w-5 flex-shrink-0 mt-0.5', style.text)} />

      {/* Content */}
      <div className="flex-1 min-w-0 pr-6">
        <p className={cn('text-sm font-medium', style.text)}>{toast.title}</p>
        <p className="text-sm text-white/70 mt-1 line-clamp-2">{truncated}</p>
        {hasDetails && (
          <p className="text-xs text-white/40 mt-2">Click to view details</p>
        )}
      </div>

      {/* Close button */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          dismiss();
        }}
        className="absolute top-3 right-3 flex h-6 w-6 items-center justify-center rounded-md text-white/40 hover:bg-white/[0.08] hover:text-white/70 transition-colors"
      >
        <X className="h-3.5 w-3.5" />
      </button>

      {/* Progress bar */}
      <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-white/[0.04]">
        <div
          className={cn('h-full transition-none', style.progress)}
          style={{ width: `${progress}%` }}
        />
      </div>
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Details Modal Component
// ─────────────────────────────────────────────────────────────────────────────

interface DetailsModalProps {
  message: string;
  onClose: () => void;
}

function DetailsModal({ message, onClose }: DetailsModalProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(message);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Close on escape
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [onClose]);

  return (
    <>
      <div
        className="fixed inset-0 z-[100] bg-black/60 backdrop-blur-sm animate-fade-in"
        onClick={onClose}
      />
      <div
        className="fixed left-1/2 top-1/2 z-[101] -translate-x-1/2 -translate-y-1/2 w-full max-w-lg animate-scale-in"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="rounded-xl bg-[#1c1c1e]/95 border border-white/[0.08] shadow-2xl">
          {/* Header */}
          <div className="flex items-center justify-between p-4 border-b border-white/[0.06]">
            <h2 className="font-semibold text-white">Details</h2>
            <button
              onClick={onClose}
              className="flex h-8 w-8 items-center justify-center rounded-lg text-white/50 hover:bg-white/[0.06] hover:text-white transition-colors"
            >
              <X className="h-4 w-4" />
            </button>
          </div>

          {/* Content */}
          <div className="p-4">
            <div className="rounded-lg bg-white/[0.02] border border-white/[0.06] p-4 max-h-[300px] overflow-y-auto">
              <pre className="text-sm text-white/80 whitespace-pre-wrap break-words font-mono">
                {message}
              </pre>
            </div>
          </div>

          {/* Footer */}
          <div className="flex justify-end gap-2 p-4 border-t border-white/[0.06]">
            <button
              onClick={handleCopy}
              className={cn(
                'flex items-center gap-2 px-3 py-2 rounded-lg text-sm transition-colors',
                copied
                  ? 'bg-emerald-500/10 text-emerald-400'
                  : 'bg-white/[0.04] text-white/60 hover:bg-white/[0.08] hover:text-white'
              )}
            >
              {copied ? (
                <>
                  <Check className="h-4 w-4" />
                  Copied
                </>
              ) : (
                <>
                  <Copy className="h-4 w-4" />
                  Copy
                </>
              )}
            </button>
            <button
              onClick={onClose}
              className="px-3 py-2 rounded-lg text-sm bg-white/[0.04] text-white/60 hover:bg-white/[0.08] hover:text-white transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Toast Container
// ─────────────────────────────────────────────────────────────────────────────

interface ToastContainerProps {
  toasts: Toast[];
  onDismiss: (id: string) => void;
  onShowDetails: (message: string) => void;
}

function ToastContainer({ toasts, onDismiss, onShowDetails }: ToastContainerProps) {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col-reverse gap-2">
      {toasts.map((t) => (
        <ToastItem
          key={t.id}
          toast={t}
          onDismiss={onDismiss}
          onShowDetails={onShowDetails}
        />
      ))}
    </div>
  );
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider
// ─────────────────────────────────────────────────────────────────────────────

let toastIdCounter = 0;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [detailsMessage, setDetailsMessage] = useState<string | null>(null);

  const addToast = useCallback((type: ToastType, message: string, title?: string) => {
    const id = `toast-${++toastIdCounter}`;
    const style = toastStyles[type];
    const newToast: Toast = {
      id,
      type,
      title: title ?? style.defaultTitle,
      message,
      duration: toastDurations[type],
    };
    setToasts((prev) => [...prev, newToast]);
  }, []);

  // Set global handler for standalone toast function
  useEffect(() => {
    globalAddToast = addToast;
    return () => {
      globalAddToast = null;
    };
  }, [addToast]);

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const showSuccess = useCallback(
    (message: string, title?: string) => {
      addToast('success', message, title);
    },
    [addToast]
  );

  const showError = useCallback(
    (message: string, title?: string) => {
      addToast('error', message, title);
    },
    [addToast]
  );

  const showInfo = useCallback(
    (message: string, title?: string) => {
      addToast('info', message, title);
    },
    [addToast]
  );

  return (
    <ToastContext.Provider value={{ showSuccess, showError, showInfo }}>
      {children}
      <ToastContainer
        toasts={toasts}
        onDismiss={dismissToast}
        onShowDetails={setDetailsMessage}
      />
      {detailsMessage && (
        <DetailsModal message={detailsMessage} onClose={() => setDetailsMessage(null)} />
      )}
    </ToastContext.Provider>
  );
}

// Re-export for backwards compatibility with error-toast.tsx imports
export { ToastProvider as ErrorToastProvider };
