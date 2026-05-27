'use client';

import { Copy, Download, Eye, EyeOff } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  collectQuoteDiagnostics,
  exportDiagnosticsAsCsv,
  exportDiagnosticsAsJson,
  formatDiagnosticsForDisplay,
  generateRequestId,
  redactSensitiveFields,
  type QuoteDiagnostics,
} from '@/lib/diagnostics';
import type { PriceQuote } from '@/types';
import { toast } from 'sonner';

interface DiagnosticsPanelProps {
  quote: PriceQuote | undefined;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
}

export function DiagnosticsPanel({
  quote,
  isOpen,
  onOpenChange,
}: DiagnosticsPanelProps) {
  const [exportFormat, setExportFormat] = useState<'json' | 'csv'>('json');
  const [showSensitiveFields, setShowSensitiveFields] = useState(false);
  const [requestId] = useState(() => generateRequestId());

  if (!quote) {
    return (
      <Dialog open={isOpen} onOpenChange={onOpenChange}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Diagnostics Panel</DialogTitle>
          </DialogHeader>
          <div className="py-8 text-center text-muted-foreground">
            No quote data available. Request a quote to view diagnostics.
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  const diagnostics = collectQuoteDiagnostics(quote, requestId);
  const displayText = formatDiagnosticsForDisplay(diagnostics);
  const finalDisplayText = showSensitiveFields
    ? displayText
    : redactSensitiveFields(displayText);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(finalDisplayText);
      toast.success('Diagnostics copied to clipboard');
    } catch {
      toast.error('Failed to copy to clipboard');
    }
  };

  const handleExport = async () => {
    let content: string;
    let filename: string;
    let mimeType: string;

    if (exportFormat === 'json') {
      content = exportDiagnosticsAsJson(diagnostics);
      filename = `diagnostics_${requestId}.json`;
      mimeType = 'application/json';
    } else {
      content = exportDiagnosticsAsCsv(diagnostics);
      filename = `diagnostics_${requestId}.csv`;
      mimeType = 'text/csv';
    }

    try {
      const blob = new Blob([content], { type: mimeType });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      URL.revokeObjectURL(url);
      toast.success(`Exported as ${exportFormat.toUpperCase()}`);
    } catch {
      toast.error('Failed to export diagnostics');
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Diagnostics Panel</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <Card className="bg-muted/30 p-4">
            <div className="font-mono text-xs whitespace-pre-wrap break-words">
              {finalDisplayText}
            </div>
          </Card>

          <div className="flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={handleCopy}
              className="gap-2"
            >
              <Copy className="h-4 w-4" />
              Copy
            </Button>

            <Button
              size="sm"
              variant="outline"
              onClick={() => setShowSensitiveFields(!showSensitiveFields)}
              className="gap-2"
            >
              {showSensitiveFields ? (
                <EyeOff className="h-4 w-4" />
              ) : (
                <Eye className="h-4 w-4" />
              )}
              {showSensitiveFields ? 'Hide' : 'Show'} Sensitive
            </Button>

            <div className="flex gap-2">
              <Select value={exportFormat} onValueChange={(value: any) => setExportFormat(value)}>
                <SelectTrigger className="w-24">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="json">JSON</SelectItem>
                  <SelectItem value="csv">CSV</SelectItem>
                </SelectContent>
              </Select>
              <Button
                size="sm"
                variant="outline"
                onClick={handleExport}
                className="gap-2"
              >
                <Download className="h-4 w-4" />
                Export
              </Button>
            </div>
          </div>

          <p className="text-xs text-muted-foreground">
            Request ID: <code>{requestId}</code>
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
