"use client";

import { Settings } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";

interface SlippageControlProps {
  slippage: number;
  onChange: (value: number) => void;
}

export function SlippageControl({ slippage, onChange }: SlippageControlProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="h-11 w-11 rounded-full">
          <Settings className="h-4 w-4 text-muted-foreground" />
          <span className="sr-only">Settings</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" avoidCollisions className="w-[calc(100vw-24px)] max-w-[240px]">
        <DropdownMenuLabel>Transaction Settings</DropdownMenuLabel>
        <DropdownMenuSeparator />
        <div className="p-3">
          <div className="text-sm font-medium mb-3">Slippage Tolerance</div>
          <div className="flex gap-2">
            {[0.1, 0.5, 1.0].map((val) => (
              <Button
                key={val}
                variant={slippage === val ? "default" : "outline"}
                size="sm"
                className="flex-1 min-h-[44px]"
                onClick={() => onChange(val)}
              >
                {val}%
              </Button>
            ))}
          </div>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
