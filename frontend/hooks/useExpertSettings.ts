'use client';

import { useState, useEffect, useCallback } from 'react';

export interface ExpertSettings {
  expertMode: boolean;
  bypassConfirmation: boolean;
  extendedRouteDetails: boolean;
}

export function useExpertSettings() {
  const [expertMode, setExpertMode] = useState(false);
  const [bypassConfirmation, setBypassConfirmation] = useState(false);
  const [extendedRouteDetails, setExtendedRouteDetails] = useState(false);
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      try {
        const storedExpert = localStorage.getItem('stellarroute.settings.expertMode') === 'true';
        const storedBypass = localStorage.getItem('stellarroute.settings.bypassConfirmation') === 'true';
        const storedExtended = localStorage.getItem('stellarroute.settings.extendedRouteDetails') === 'true';

        queueMicrotask(() => {
          setExpertMode(storedExpert);
          setBypassConfirmation(storedExpert ? storedBypass : false);
          setExtendedRouteDetails(storedExpert ? storedExtended : false);
          setIsHydrated(true);
        });
      } catch (e) {
        console.error('Failed to load expert settings from localStorage', e);
        queueMicrotask(() => {
          setIsHydrated(true);
        });
      }
    }
  }, []);

  const updateExpertMode = useCallback((val: boolean) => {
    setExpertMode(val);
    try {
      localStorage.setItem('stellarroute.settings.expertMode', String(val));
      if (!val) {
        setBypassConfirmation(false);
        setExtendedRouteDetails(false);
        localStorage.setItem('stellarroute.settings.bypassConfirmation', 'false');
        localStorage.setItem('stellarroute.settings.extendedRouteDetails', 'false');
      }
    } catch (e) {
      console.error('Failed to persist expertMode to localStorage', e);
    }
  }, []);

  const updateBypassConfirmation = useCallback((val: boolean) => {
    setBypassConfirmation(val);
    try {
      localStorage.setItem('stellarroute.settings.bypassConfirmation', String(val));
    } catch (e) {
      console.error('Failed to persist bypassConfirmation to localStorage', e);
    }
  }, []);

  const updateExtendedRouteDetails = useCallback((val: boolean) => {
    setExtendedRouteDetails(val);
    try {
      localStorage.setItem('stellarroute.settings.extendedRouteDetails', String(val));
    } catch (e) {
      console.error('Failed to persist extendedRouteDetails to localStorage', e);
    }
  }, []);

  return {
    expertMode,
    bypassConfirmation,
    extendedRouteDetails,
    updateExpertMode,
    updateBypassConfirmation,
    updateExtendedRouteDetails,
    isHydrated,
  };
}
