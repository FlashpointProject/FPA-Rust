import { Component, PropsWithChildren } from "react";

type ErrorBoundaryProps = {
    message?: string;
} & PropsWithChildren;

type ErrorBoundaryState = {
    hasError: boolean;
    error?: any;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
    constructor(props: ErrorBoundaryProps) {
        super(props);
        this.state = { hasError: false };
    }

    static getDerivedStateFromError(error) {
        // Update state so the next render will show the fallback UI.
        return { hasError: true, error };
    }

    componentDidCatch(error, errorInfo) {
        // You can also log the error to an error reporting service
        console.log(error, errorInfo);
    }

    render() {
        if (this.state.hasError) {
            // You can render any custom fallback UI
            return (
                <div>
                    <div className="text-xl">
                        {this.props.message ? this.props.message : 'Something went wrong:'}
                    </div>
                    <div>
                        {this.state.error?.message}
                    </div>
                </div>
            );
        }

        return this.props.children;
    }
}