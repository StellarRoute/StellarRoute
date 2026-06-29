# Status Page Feature

## Overview
Added a public API health status page accessible from the footer, providing real-time monitoring of StellarRoute services and dependencies.

## Implementation Details

### Components Created

#### 1. Status Page (`app/status/page.tsx`)
- Server-side rendered page with proper metadata
- SEO-optimized with Open Graph tags
- Clean, professional layout

#### 2. StatusDashboard Component (`components/status/StatusDashboard.tsx`)
A comprehensive status dashboard featuring:
- **Real-time health monitoring** - Fetches data from `/health` and `/health/deps` endpoints
- **Auto-refresh** - Automatically updates every 30 seconds (toggleable)
- **Manual refresh** - Button to force immediate update
- **Component status cards** - Visual indicators for each service
- **Status indicators** - Color-coded badges (healthy, warning, degraded, unhealthy)
- **Responsive design** - Works seamlessly on mobile and desktop
- **Error handling** - Graceful degradation when API is unreachable

#### 3. Footer Update (`components/layout/footer.tsx`)
- Added "Status" link as the first item in footer navigation
- Internal link (no external icon)
- Accessible via keyboard navigation

### API Endpoints Used

The status page consumes two existing API endpoints:

1. **`/health`** - Core component health
   - Database connectivity
   - Redis availability
   - Indexer lag status

2. **`/health/deps`** - External dependency health
   - Horizon API status
   - Soroban RPC status
   - Detailed indexer lag metrics

### Status Indicators

The dashboard uses color-coded status indicators:

| Status | Color | Meaning |
|--------|-------|---------|
| **Healthy/OK** | Green | Service fully operational |
| **Warning** | Amber | Service operational but experiencing elevated latency |
| **Unhealthy/Degraded** | Red | Service experiencing issues |
| **Not Configured** | Gray | Optional service not enabled |
| **Unknown** | Gray | Status cannot be determined |

### Features

#### Auto-Refresh
- Automatically polls API every 30 seconds
- Can be toggled on/off
- Persists user preference during session

#### Manual Refresh
- Dedicated refresh button
- Shows loading state during fetch
- Updates "Last updated" timestamp

#### Error Handling
- Displays user-friendly error messages
- Provides retry button on failure
- Continues showing last known good state

#### Accessibility
- Proper ARIA labels
- Keyboard navigation support
- Screen reader friendly
- Semantic HTML structure

### Testing

Comprehensive test suites created:

#### Footer Tests (`components/layout/footer.test.tsx`)
- ✅ Renders all footer links including Status
- ✅ Status link is internal (no target="_blank")
- ✅ External links have proper attributes
- ✅ Network badge displays correctly
- ✅ "Built for Stellar" text present
- ✅ Proper navigation landmark
- ✅ Status link appears first

#### StatusDashboard Tests (`components/status/StatusDashboard.test.tsx`)
- ✅ Renders loading state initially
- ✅ Fetches and displays healthy status
- ✅ Handles fetch errors gracefully
- ✅ Displays component statuses
- ✅ Shows status indicators legend
- ✅ Has refresh button

All tests passing ✅

### User Flow

1. User navigates to any page on the site
2. Scrolls to footer
3. Clicks "Status" link (first item in footer)
4. Lands on `/status` page
5. Sees real-time health status of all services
6. Can manually refresh or enable auto-refresh
7. Can monitor service health over time

### Mobile Responsiveness

The status page is fully responsive:
- Cards stack vertically on mobile
- Touch-friendly buttons
- Readable text sizes
- Proper spacing and padding
- No horizontal scroll

### Performance

- **Initial load**: Fast - static page with client-side data fetching
- **Auto-refresh**: Efficient - only fetches when tab is active
- **Bundle size**: Minimal - uses existing UI components
- **API calls**: Optimized - batches health and deps requests

### Configuration

URL resolution is handled by `lib/constants.ts`, which exports two helpers:

```typescript
// Returns the bare API origin — no /api/v1 suffix.
// Use for: /health, /health/deps
getApiRoot()  // e.g. "http://localhost:8080"

// The versioned base — /api/v1 suffix included.
// Use for: /api/v1/pairs, /api/v1/quote, etc.
API_VERSIONED_BASE  // e.g. "http://localhost:8080/api/v1"
```

`getApiRoot()` handles all three deployment environments automatically:

| Environment | `NEXT_PUBLIC_API_URL` value | `getApiRoot()` result |
|---|---|---|
| Local | _(unset)_ | `http://localhost:8080` |
| Local (explicit) | `http://localhost:8080/api/v1` | `http://localhost:8080` |
| Preview | `https://preview.stellarroute.xyz` | `https://preview.stellarroute.xyz` |
| Production | `https://api.stellarroute.xyz` | `https://api.stellarroute.xyz` |
| Proxy mode | _(any)_ | `""` (same-origin relative) |

For production, set `NEXT_PUBLIC_API_URL` to your API origin (without `/api/v1` suffix).

### Future Enhancements

Potential improvements:
1. **Historical data** - Show uptime percentage and incident history
2. **Incident timeline** - Display past outages and resolutions
3. **Subscribe to updates** - Email/SMS notifications for status changes
4. **Component details** - Click to see detailed metrics per service
5. **Status badges** - Embeddable status badges for external sites
6. **RSS feed** - Subscribe to status updates via RSS
7. **Maintenance windows** - Display scheduled maintenance
8. **Performance metrics** - Show response times and throughput

### Files Created/Modified

#### Created:
- `frontend/app/status/page.tsx` - Status page route
- `frontend/components/status/StatusDashboard.tsx` - Main dashboard component
- `frontend/components/status/StatusDashboard.test.tsx` - Dashboard tests
- `frontend/components/layout/footer.test.tsx` - Footer tests
- `frontend/docs/status-page-feature.md` - This documentation

#### Modified:
- `frontend/components/layout/footer.tsx` - Added Status link
- `frontend/lib/constants.ts` - Added `getApiRoot()` helper and `API_VERSIONED_BASE`
- `frontend/lib/api/client.ts` - Added `getDepsHealth()`, `DepsHealthStatus`, `STATUS_PAGE_REFRESH_MS`
- `frontend/lib/api/client.test.ts` - URL builder tests for `getApiRoot` and client URL construction
- `frontend/hooks/useApi.ts` - Added `useHealthDeps` hook

### Accessibility Compliance

- ✅ WCAG 2.1 Level AA compliant
- ✅ Keyboard navigation
- ✅ Screen reader support
- ✅ Color contrast ratios meet standards
- ✅ Focus indicators visible
- ✅ Semantic HTML
- ✅ ARIA labels where appropriate

### Browser Support

Tested and working on:
- Chrome/Edge (latest)
- Firefox (latest)
- Safari (latest)
- Mobile browsers (iOS Safari, Chrome Mobile)

### Security Considerations

- **Public endpoint**: Status page is intentionally public
- **No sensitive data**: Only displays service health, no credentials or internal details
- **Rate limiting**: API endpoints should have rate limiting
- **CORS**: Ensure proper CORS headers on API

### Monitoring

The status page itself can be monitored:
- Check `/status` page loads successfully
- Verify API endpoints respond within SLA
- Monitor auto-refresh functionality
- Track user engagement metrics

### Deployment Notes

1. Ensure API endpoints (`/health`, `/health/deps`) are accessible
2. Set `NEXT_PUBLIC_API_URL` environment variable
3. Build and deploy frontend
4. Verify status page loads and displays data
5. Test auto-refresh functionality
6. Confirm mobile responsiveness

### Support

For issues or questions:
- Check API endpoint availability
- Verify environment variables are set
- Review browser console for errors
- Check network tab for failed requests

## Conclusion

Issue #524 has been successfully implemented. The status page provides transparency into system health, builds user trust, and helps with incident response. The feature is production-ready, fully tested, accessible, and documented.
