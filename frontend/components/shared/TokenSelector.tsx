import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Card } from '@/components/ui/card';

export interface TokenOption {
  value: string;
  label: string;
  symbol: string;
}

export interface TokenSelectorProps {
  value?: string;
  options: TokenOption[];
  loading?: boolean;
  error?: string;
  placeholder?: string;
  onChange: (value: string) => void;
}

export function TokenSelector({ value, options, loading, error, placeholder = 'Select a token', onChange }: TokenSelectorProps) {
  if (loading) {
    return (
      <Card className="p-4">
        <p className="text-sm text-muted-foreground">Loading tokens…</p>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-4 border-destructive">
        <p className="text-sm text-destructive">Token selector error: {error}</p>
      </Card>
    );
  }

  if (!options.length) {
    return (
      <Card className="p-4">
        <p className="text-sm text-muted-foreground">No tokens available</p>
      </Card>
    );
  }

  return (
    <Select value={value} onValueChange={onChange}>
      <SelectTrigger className="w-full">
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {options.map((token) => (
          <SelectItem key={token.value} value={token.value}>
            {token.symbol} · {token.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
