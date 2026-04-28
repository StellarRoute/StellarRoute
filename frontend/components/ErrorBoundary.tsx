"use client";

import React, { Component, ReactNode } from "react";
import { Button } from "@/components/ui/button";

import { AlertCircle, RefreshCcw, Home } from "lucide-react";

type Props = {
  children: ReactNode;
};

type State = {
  hasError: boolean;
  error: Error | null;
};

class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // You could also log the error to an error reporting service here
    console.error("ErrorBoundary caught an error:", error, errorInfo);
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null });
    window.location.reload();
  };

  handleGoHome = () => {
    this.setState({ hasError: false, error: null });
    window.location.href = "/";
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="min-h-screen w-full flex items-center justify-center p-4 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-slate-900 via-slate-950 to-black overflow-hidden relative">
          {/* Animated Background Blobs */}
          <div className="absolute top-[-10%] left-[-10%] w-[40%] h-[40%] bg-primary/20 rounded-full blur-[120px] animate-pulse" />
          <div className="absolute bottom-[-10%] right-[-10%] w-[40%] h-[40%] bg-blue-500/10 rounded-full blur-[120px] animate-pulse delay-1000" />
          
          <div className="max-w-md w-full relative z-10">
            <div className="bg-white/5 backdrop-blur-2xl border border-white/10 rounded-[2.5rem] p-8 shadow-2xl overflow-hidden relative group">
              <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-50" />
              
              <div className="relative z-10 flex flex-col items-center text-center">
                <div className="mb-6 relative">
                  <div className="absolute inset-0 bg-destructive/20 blur-2xl rounded-full animate-pulse" />
                  <div className="h-20 w-20 bg-destructive/10 border border-destructive/20 rounded-3xl flex items-center justify-center relative backdrop-blur-sm">
                    <AlertCircle className="h-10 w-10 text-destructive" />
                  </div>
                </div>

                <h1 className="text-3xl font-bold text-white mb-3 tracking-tight">
                  Oops! Something went wrong
                </h1>
                
                <p className="text-slate-400 mb-8 leading-relaxed">
                  We've encountered an unexpected error. Don't worry, our team has been notified. 
                  Try refreshing the page or going back to the dashboard.
                </p>

                {this.state.error && (
                  <div className="w-full mb-8 p-4 bg-black/40 rounded-2xl border border-white/5 text-left overflow-hidden">
                    <p className="text-[10px] uppercase tracking-widest font-bold text-slate-500 mb-2">Error Details</p>
                    <p className="text-xs font-mono text-slate-300 break-words line-clamp-3 italic">
                      {this.state.error.message || "An unknown error occurred"}
                    </p>
                  </div>
                )}

                <div className="grid grid-cols-2 gap-4 w-full">
                  <Button 
                    variant="outline" 
                    className="h-12 rounded-2xl border-white/10 bg-white/5 hover:bg-white/10 text-white transition-all duration-300"
                    onClick={this.handleGoHome}
                  >
                    <Home className="mr-2 h-4 w-4" />
                    Home
                  </Button>
                  <Button 
                    className="h-12 rounded-2xl bg-primary hover:bg-primary/90 text-primary-foreground shadow-lg shadow-primary/20 transition-all duration-300"
                    onClick={this.handleReset}
                  >
                    <RefreshCcw className="mr-2 h-4 w-4" />
                    Refresh
                  </Button>
                </div>
              </div>
            </div>
            
            <p className="text-center text-[10px] text-slate-500 mt-6 uppercase tracking-[0.2em] font-medium">
              StellarRoute Resilience System
            </p>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export default ErrorBoundary;