import type { MDXComponents } from "mdx/types";
const FORMAT_H1 = "mt-2 scroll-m-20 text-4xl font-bold tracking-tight"
const FORMAT_H2 = "mt-10 scroll-m-20 border-b border-b-slate-200 pb-1 text-3xl font-semibold tracking-tight first:mt-0"
const FORMAT_H3 = "mt-8 scroll-m-20 text-2xl font-semibold tracking-tight"
const FORMAT_H4 = "mt-8 scroll-m-20 text-xl font-semibold tracking-tight"
const FORMAT_H5 = "mt-8 scroll-m-20 text-lg font-semibold tracking-tight"
const FORMAT_H6 = "mt-8 scroll-m-20 text-base font-semibold tracking-tight"
const FORMAT_A = "font-medium text-blue-500 dark:text-blue-400 underline underline-offset-4"

export const markdownComponents: MDXComponents = {
    h1: ({ className, ...props }) => (
        <h1 className={`${FORMAT_H1} ${className}`} {...props} />
    ),
    h2: ({ className, ...props }) => (
        <h2 className={`${FORMAT_H2} ${className}`} {...props} />
    ),
    h3: ({ className, ...props }) => (
        <h3 className={`${FORMAT_H3} ${className}`} {...props} />
    ),
    h4: ({ className, ...props }) => (
        <h4 className={`${FORMAT_H4} ${className}`} {...props} />
    ),
    h5: ({ className, ...props }) => (
        <h5 className={`${FORMAT_H5} ${className}`} {...props} />
    ),
    h6: ({ className, ...props }) => (
        <h6 className={`${FORMAT_H6} ${className}`} {...props} />
    ),
    a: ({ className, ...props }) => (
        <a className={`${FORMAT_A} ${className}`} {...props} />
    ),
}