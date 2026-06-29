# API Key Rotation

StellarRoute uses a simple environment-variable based system for API keys.

## Configuring Keys
API keys are provided via the `API_KEYS` environment variable as a comma-separated list.

```bash
export API_KEYS="secret-key-1,secret-key-2"
export REQUIRE_AUTH="true"
```

## Rotating Keys
To rotate an API key without downtime:
1. Append the new key to the `API_KEYS` environment variable:
   `API_KEYS="old-key,new-key"`
2. Restart the application servers or trigger a rolling deployment so the new key becomes active.
3. Update the client integration to use `new-key`.
4. Once the client has completely switched to `new-key`, remove `old-key` from `API_KEYS`:
   `API_KEYS="new-key"`
5. Perform another rolling deployment to revoke `old-key`.
