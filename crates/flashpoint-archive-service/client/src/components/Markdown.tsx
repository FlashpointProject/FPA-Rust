import { compile, run } from "@mdx-js/mdx";
import { MDXProvider } from "@mdx-js/react";
import * as provider from '@mdx-js/react'
import React, { useEffect, useMemo, useState } from "react";
import * as runtime from 'react/jsx-runtime';
import { markdownComponents } from "../markdownComponents";
import remarkGfm from "remark-gfm";
import { ErrorBoundary } from "./ErrorBoundary";
import rehypeStarryNight from "rehype-starry-night";

export type MarkdownProps = {
    content: string;
    innerProps?: Record<string, any>;
}

export function Markdown(props: MarkdownProps) {
    const [Content, setContent] = useState<React.ComponentType | null>(null);
    const innerProps = props.innerProps ? props.innerProps : {};

    useEffect(() => {
        const loadMdx = async () => {
            try {
                const code = await compile(props.content, {
                    outputFormat: "function-body",
                    providerImportSource: "@mdx-js/react",
                    remarkPlugins: [remarkGfm],
                    rehypePlugins: [rehypeStarryNight],
                });
                const element = await run(code, { ...runtime, useMDXComponents: () => markdownComponents });
                setContent(() => element.default);
            } catch (err) {
                setContent(() => <div>{err}</div>)
            }
        };

        loadMdx();
    }, [props.content]);

    return (
        <div className="h-full w-full py-2">
            <div className="prose dark:prose-invert">
                {Content ? <Content {...innerProps} /> : <p>Loading...</p>}
            </div>
        </div>
    );
}

export type MarkdownEditorProps = {
    value: string;
    placeholder?: string;
    rows?: number;
    onChange: (value: string) => void;
    innerProps?: Record<string, any>;
}


export function MarkdownEditor(props: MarkdownEditorProps) {
    const [previewMode, setPreviewMode] = useState(false);

    return (
        <div className="mr-12">
            <div className="flex flex-row">
                <button className="bg-blue-500 text-white border-blue-600 rounded p-2 font-semibold" onClick={() => setPreviewMode(!previewMode)}>
                    {previewMode ? 'Switch to Edit Mode' : 'Switch to Preview Mode'}
                </button>
            </div>
            {previewMode ? (
                <ErrorBoundary message="Error rendering MDX:">
                    <Markdown
                        content={props.value}
                        innerProps={props.innerProps} />
                </ErrorBoundary>
            ) : (
                <textarea
                    className="w-full bg-white text-black placeholder-gray-500 dark:bg-gray-800 dark:text-white dark:placeholder-gray-400 p-2 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                    value={props.value}
                    placeholder={props.placeholder}
                    onChange={(event) => props.onChange(event.target.value)}
                    rows={props.rows || 5}
                    style={{ minHeight: '3rem', overflowY: 'auto' }} />
            )}
        </div>
    )
}
