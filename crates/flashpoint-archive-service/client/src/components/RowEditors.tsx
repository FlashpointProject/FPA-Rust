export type RowEditorTextAreaProps = {
    value: string;
    placeholder?: string;
    rows?: number;
    onChange: (value: string) => void;
}

export function RowEditorInput(props: RowEditorTextAreaProps) {
    return (
        <div className="w-full border-2 border-gray-300 dark:border-gray-600 rounded-md mt-1 mb-1">
            <input
                className="w-full bg-white text-black placeholder-gray-500 dark:bg-gray-800 dark:text-white dark:placeholder-gray-400 p-2 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                type="text"
                value={props.value}
                placeholder={props.placeholder}
                onChange={(event) => props.onChange(event.target.value)} />
        </div>

    )
}

export function RowEditorTextArea(props: RowEditorTextAreaProps) {
    return (
        <div className="w-full border-2 border-gray-300 dark:border-gray-600 rounded-md mt-1 mb-1">
            <textarea
                className="w-full bg-white text-black placeholder-gray-500 dark:bg-gray-800 dark:text-white dark:placeholder-gray-400 p-2 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 dark:focus:ring-blue-400"
                value={props.value}
                placeholder={props.placeholder}
                onChange={(event) => props.onChange(event.target.value)}
                rows={props.rows || 5}
                style={{ minHeight: '3rem', overflowY: 'auto' }}
            />
        </div>
    )
}