/**
 * useProtocolEvents Hook
 * 
 * Custom hook for managing protocol event monitoring, filtering,
 * and real-time event updates from the backend API
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { toast } from 'react-hot-toast';
import { 
  apiClient, 
  ProtocolEvent, 
  EventStats 
} from '../lib/api-client';

export interface EventFilters {
  protocol?: string;
  event_type?: string;
  severity?: string;
  from_date?: string;
  to_date?: string;
  page?: number;
  per_page?: number;
}

export interface UseProtocolEventsReturn {
  // Events Data
  events: ProtocolEvent[];
  eventStats: EventStats | null;
  currentEvent: ProtocolEvent | null;
  
  // Pagination
  totalEvents: number;
  currentPage: number;
  perPage: number;
  hasNextPage: boolean;
  hasPrevPage: boolean;
  
  // Loading States
  isLoadingEvents: boolean;
  isLoadingStats: boolean;
  
  // Filters
  filters: EventFilters;
  setFilters: (filters: EventFilters) => void;
  clearFilters: () => void;
  
  // Actions
  loadEvents: (filters?: EventFilters) => Promise<void>;
  loadEventStats: (filters?: Omit<EventFilters, 'page' | 'per_page'>) => Promise<void>;
  getEvent: (eventId: string) => Promise<void>;
  refreshEvents: () => Promise<void>;
  
  // Pagination
  goToPage: (page: number) => void;
  nextPage: () => void;
  prevPage: () => void;
  
  // Real-time Updates
  isConnected: boolean;
  lastEventUpdate: Date | null;
  
  // Error Handling
  error: string | null;
  clearError: () => void;
}

export const useProtocolEvents = (initialFilters?: EventFilters): UseProtocolEventsReturn => {
  // State Management
  const [events, setEvents] = useState<ProtocolEvent[]>([]);
  const [eventStats, setEventStats] = useState<EventStats | null>(null);
  const [currentEvent, setCurrentEvent] = useState<ProtocolEvent | null>(null);
  const [totalEvents, setTotalEvents] = useState(0);
  const [currentPage, setCurrentPage] = useState(1);
  const [perPage, setPerPage] = useState(20);
  const [isLoadingEvents, setIsLoadingEvents] = useState(false);
  const [isLoadingStats, setIsLoadingStats] = useState(false);
  const [filters, setFiltersState] = useState<EventFilters>(initialFilters || {});
  const [isConnected, setIsConnected] = useState(false);
  const [lastEventUpdate, setLastEventUpdate] = useState<Date | null>(null);
  const [error, setError] = useState<string | null>(null);

  // WebSocket connection ref
  const wsRef = useRef<WebSocket | null>(null);

  // Error handling
  const handleError = useCallback((error: any, context: string) => {
    console.error(`Protocol events error in ${context}:`, error);
    const errorMessage = error?.message || `Failed to ${context}`;
    setError(errorMessage);
    toast.error(errorMessage);
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Event Loading Functions
  const loadEvents = useCallback(async (customFilters?: EventFilters) => {
    try {
      setIsLoadingEvents(true);
      clearError();

      const searchFilters = customFilters || filters;
      const response = await apiClient.getProtocolEvents({
        ...searchFilters,
        page: searchFilters.page || currentPage,
        per_page: searchFilters.per_page || perPage,
      });

      setEvents(response.events);
      setTotalEvents(response.total);
      setCurrentPage(response.page);
      setPerPage(response.per_page);
    } catch (error) {
      handleError(error, 'load protocol events');
    } finally {
      setIsLoadingEvents(false);
    }
  }, [filters, currentPage, perPage, handleError, clearError]);

  const loadEventStats = useCallback(async (customFilters?: Omit<EventFilters, 'page' | 'per_page'>) => {
    try {
      setIsLoadingStats(true);
      clearError();

      const searchFilters = customFilters || filters;
      const stats = await apiClient.getEventStats({
        protocol_name: searchFilters.protocol,
        from_date: searchFilters.from_date,
        to_date: searchFilters.to_date,
      });

      setEventStats(stats);
    } catch (error) {
      handleError(error, 'load event statistics');
    } finally {
      setIsLoadingStats(false);
    }
  }, [filters, handleError, clearError]);

  const getEvent = useCallback(async (eventId: string) => {
    try {
      clearError();
      const event = await apiClient.getProtocolEvent(eventId);
      setCurrentEvent(event);
    } catch (error) {
      handleError(error, 'fetch protocol event');
    }
  }, [handleError, clearError]);

  const refreshEvents = useCallback(async () => {
    await Promise.all([
      loadEvents(),
      loadEventStats(),
    ]);
  }, [loadEvents, loadEventStats]);

  // Filter Management
  const setFilters = useCallback((newFilters: EventFilters) => {
    setFiltersState(newFilters);
    setCurrentPage(1); // Reset to first page when filters change
  }, []);

  const clearFilters = useCallback(() => {
    setFiltersState({});
    setCurrentPage(1);
  }, []);

  // Pagination Functions
  const goToPage = useCallback((page: number) => {
    setCurrentPage(page);
  }, []);

  const nextPage = useCallback(() => {
    if (hasNextPage) {
      setCurrentPage(prev => prev + 1);
    }
  }, []);

  const prevPage = useCallback(() => {
    if (hasPrevPage) {
      setCurrentPage(prev => prev - 1);
    }
  }, []);

  // Computed values
  const hasNextPage = currentPage * perPage < totalEvents;
  const hasPrevPage = currentPage > 1;

  // Real-time WebSocket Connection
  const connectRealTime = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    try {
      wsRef.current = apiClient.connectWebSocket(
        (data) => {
          setLastEventUpdate(new Date());
          
          switch (data.type) {
            case 'PROTOCOL_EVENT':
              // Add new event to the beginning of the list
              setEvents(prev => [data.event, ...prev.slice(0, perPage - 1)]);
              setTotalEvents(prev => prev + 1);
              
              // Show notification for high-severity events
              if (data.event.severity === 'Critical' || data.event.severity === 'High') {
                toast.error(`🚨 ${data.event.severity} Protocol Event: ${data.event.title}`, {
                  duration: 10000,
                  icon: '⚠️',
                });
              }
              break;
              
            case 'EVENT_STATS_UPDATE':
              setEventStats(data.stats);
              break;
              
            default:
              console.log('Received protocol event WebSocket message:', data);
          }
        },
        (error) => {
          console.error('Protocol events WebSocket error:', error);
          setIsConnected(false);
          toast.error('Lost connection to protocol event monitoring');
        }
      );

      wsRef.current.onopen = () => {
        setIsConnected(true);
        console.log('Connected to real-time protocol event monitoring');
      };

      wsRef.current.onclose = () => {
        setIsConnected(false);
        console.log('Disconnected from protocol event monitoring');
      };

    } catch (error) {
      handleError(error, 'connect to protocol event monitoring');
    }
  }, [perPage, handleError]);

  const disconnectRealTime = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
      setIsConnected(false);
    }
  }, []);

  // Effects

  // Load events when filters or pagination change
  useEffect(() => {
    loadEvents();
  }, [filters, currentPage, perPage]);

  // Load stats when filters change (but not pagination)
  useEffect(() => {
    loadEventStats();
  }, [filters.protocol, filters.from_date, filters.to_date]);

  // Auto-connect to real-time updates
  useEffect(() => {
    connectRealTime();
    
    return () => {
      disconnectRealTime();
    };
  }, [connectRealTime, disconnectRealTime]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      disconnectRealTime();
    };
  }, [disconnectRealTime]);

  return {
    // Events Data
    events,
    eventStats,
    currentEvent,
    
    // Pagination
    totalEvents,
    currentPage,
    perPage,
    hasNextPage,
    hasPrevPage,
    
    // Loading States
    isLoadingEvents,
    isLoadingStats,
    
    // Filters
    filters,
    setFilters,
    clearFilters,
    
    // Actions
    loadEvents,
    loadEventStats,
    getEvent,
    refreshEvents,
    
    // Pagination
    goToPage,
    nextPage,
    prevPage,
    
    // Real-time Updates
    isConnected,
    lastEventUpdate,
    
    // Error Handling
    error,
    clearError,
  };
};
