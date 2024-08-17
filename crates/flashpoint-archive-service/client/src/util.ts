// Utility function to convert a string from snake_case to camelCase
export function toCamelCase(str: string): string {
    return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

export function toSnakeCase(str: string): string {
    return str.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
}

export function convertKeysToSnakeCase<T>(obj: T): T {
    if (Array.isArray(obj)) {
        // If the object is an array, recursively convert each element
        return obj.map(item => convertKeysToSnakeCase(item)) as unknown as T;
    } else if (obj !== null && typeof obj === 'object') {
        // If the object is an object, recursively convert each key
        return Object.keys(obj).reduce((acc, key) => {
            const snakeCaseKey = toSnakeCase(key);
            const value = (obj as Record<string, unknown>)[key];
            (acc as Record<string, unknown>)[snakeCaseKey] = convertKeysToSnakeCase(value);
            return acc;
        }, {} as T);
    } else {
        // If the object is a primitive (string, number, etc.), return it as-is
        return obj;
    }
}

// Function to recursively convert object keys from snake_case to camelCase
export function convertKeysToCamelCase<T>(obj: T): T {
    if (Array.isArray(obj)) {
        // If the object is an array, recursively convert each element
        return obj.map(item => convertKeysToCamelCase(item)) as unknown as T;
    } else if (obj !== null && typeof obj === 'object') {
        // If the object is an object, recursively convert each key
        return Object.keys(obj).reduce((acc, key) => {
            const camelCaseKey = toCamelCase(key);
            const value = (obj as Record<string, unknown>)[key];
            (acc as Record<string, unknown>)[camelCaseKey] = convertKeysToCamelCase(value);
            return acc;
        }, {} as T);
    } else {
        // If the object is a primitive (string, number, etc.), return it as-is
        return obj;
    }
}

export type LoaderType<T extends (...args: any) => any> = Awaited<ReturnType<T>>;