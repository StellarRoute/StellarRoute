"use client";

import React, { useMemo, useState, useEffect, useRef } from "react";
import { Search, X, History, Check } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { AssetIcon } from "@/components/shared/AssetIcon";
import { useRecentTokens } from "@/hooks/useRecentTokens";

export interface AssetOption {
  code: string;
  asset: string;
  issuer?: string;
  displayName?: string;
}

interface TokenSearchModalProps {
  isOpen: boolean;
  onClose: () => void;
  assets: AssetOption[];
  onSelect: (asset: string) => void;
  title: string;
  selectedAsset?: string;
}

export function TokenSearchModal({
  isOpen,
  onClose,
  assets,
  onSelect,
  title,
  selectedAsset,
}: TokenSearchModalProps) {
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const { recentTokens, addRecentToken } = useRecentTokens();
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Filter assets based on search
  const filteredAssets = useMemo(() => {
    const query = search.toLowerCase().trim();
    if (!query) return assets;

    return assets.filter(
      (a) =>
        a.code.toLowerCase().includes(query) ||
        a.asset.toLowerCase().includes(query) ||
        a.issuer?.toLowerCase().includes(query) ||
        a.displayName?.toLowerCase().includes(query)
    );
  }, [assets, search]);

  // Recent assets based on the persisted IDs
  const recentAssets = useMemo(() => {
    return recentTokens
      .map((id) => assets.find((a) => a.asset === id))
      .filter((a): a is AssetOption => !!a);
  }, [recentTokens, assets]);

  // Reset selected index when search changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [search]);

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (filteredAssets.length === 0) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) => (prev + 1) % filteredAssets.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex(
        (prev) => (prev - 1 + filteredAssets.length) % filteredAssets.length
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (filteredAssets[selectedIndex]) {
        handleSelect(filteredAssets[selectedIndex].asset);
      }
    }
  };

  const handleSelect = (asset: string) => {
    addRecentToken(asset);
    onSelect(asset);
    onClose();
  };

  // Scroll active item into view
  useEffect(() => {
    if (listRef.current && selectedIndex >= 0) {
      const listElement = listRef.current.querySelector('[data-radix-scroll-area-viewport]');
      if (!listElement) return;
      
      const items = listElement.querySelectorAll('button[data-token-item]');
      const activeItem = items[selectedIndex] as HTMLElement;
      
      if (activeItem) {
        activeItem.scrollIntoView?.({ block: "nearest" });
      }
    }
  }, [selectedIndex]);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-md p-0 overflow-hidden flex flex-col h-[600px] max-h-[85vh]">
        <DialogHeader className="p-4 pb-2 border-b">
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <div className="p-4 space-y-4 flex flex-col flex-1 overflow-hidden">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              ref={inputRef}
              placeholder="Search by symbol or address..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={handleKeyDown}
              className="pl-9 h-11"
              autoFocus
            />
            {search && (
              <button
                onClick={() => setSearch("")}
                aria-label="Clear search"
                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                <X className="h-4 w-4" />
              </button>
            )}
          </div>

          {!search && recentAssets.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                <History className="h-3 w-3" />
                RECENT
              </div>
              <div className="flex flex-wrap gap-2">
                {recentAssets.map((asset) => (
                  <Button
                    key={`recent-${asset.asset}`}
                    variant="outline"
                    size="sm"
                    className="h-8 py-1 px-3 gap-2"
                    onClick={() => handleSelect(asset.asset)}
                  >
                    <AssetIcon
                      symbol={asset.code}
                      className="size-5 border-border/40 bg-primary/5 text-[0.55rem]"
                    />
                    <span className="font-medium">{asset.code}</span>
                  </Button>
                ))}
              </div>
            </div>
          )}

          <div className="flex-1 overflow-hidden flex flex-col min-h-0">
            <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center justify-between">
              <span>TOKENS</span>
              <span>{filteredAssets.length} results</span>
            </div>
            <ScrollArea className="flex-1 -mx-4 px-4" ref={listRef}>
              <div className="space-y-1 pb-4">
                {filteredAssets.length === 0 ? (
                  <div className="py-20 text-center">
                    <p className="text-muted-foreground">No tokens found</p>
                    <p className="text-xs text-muted-foreground/60 mt-1">
                      Try searching by contract address or another symbol
                    </p>
                  </div>
                ) : (
                  filteredAssets.map((asset, index) => (
                    <button
                      key={asset.asset}
                      data-token-item
                      className={cn(
                        "w-full flex items-center justify-between p-3 rounded-lg transition-all text-left outline-none",
                        selectedIndex === index
                          ? "bg-accent text-accent-foreground"
                          : "hover:bg-accent/50",
                        selectedAsset === asset.asset &&
                          "ring-1 ring-primary/50 bg-primary/5"
                      )}
                      onClick={() => handleSelect(asset.asset)}
                      onMouseEnter={() => setSelectedIndex(index)}
                    >
                      <div className="flex min-w-0 items-center gap-3">
                        <AssetIcon
                          symbol={asset.code}
                          className="size-9 border-border/50 bg-primary/5 text-xs"
                        />
                        <div className="flex min-w-0 flex-col">
                          <span className="font-bold underline decoration-primary/20 decoration-2 underline-offset-2">
                            {asset.code}
                          </span>
                          {asset.displayName &&
                            asset.displayName !== asset.code && (
                              <span className="text-xs text-muted-foreground truncate max-w-[150px]">
                                {asset.displayName}
                              </span>
                            )}
                          {asset.issuer && (
                            <span className="w-full max-w-[200px] truncate text-[10px] font-mono text-muted-foreground">
                              {asset.issuer}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {selectedAsset === asset.asset && (
                          <Check className="h-4 w-4 text-primary" />
                        )}
                      </div>
                    </button>
                  ))
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
