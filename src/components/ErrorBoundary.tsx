import { Component, type ReactNode } from "react";

export class ErrorBoundary extends Component<{ fallback: (error: Error, reset: () => void) => ReactNode; children: ReactNode }, { error: Error | null }> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error) {
    console.error(error);
  }

  render() {
    const { error } = this.state;
    return error ? this.props.fallback(error, () => this.setState({ error: null })) : this.props.children;
  }
}
