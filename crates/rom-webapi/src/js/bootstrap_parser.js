    class DOMParser {
        parseFromString(input, mimeType) {
            const source = String(input ?? "");
            const normalizedMimeType = String(mimeType);

            if (normalizedMimeType === "text/html") {
                return parseHtmlDocument(source, normalizedMimeType);
            }

            if (
                normalizedMimeType === "text/xml" ||
                normalizedMimeType === "application/xml" ||
                normalizedMimeType === "application/xhtml+xml" ||
                normalizedMimeType === "image/svg+xml"
            ) {
                return parseXmlDocument(source, normalizedMimeType);
            }

            throw new TypeError("Unsupported DOMParser mimeType");
        }
    }

    function parseHtmlDocument(source, mimeType) {
        const document = new Document();
        document.contentType = mimeType;
        const parsed = parseMarkup(document, source, false);

        if (parsed.error) {
            document.body.textContent = source.replace(/<[^>]*>/g, "");
            return document;
        }

        const htmlElement = parsed.nodes.find((node) => node.tagName === "HTML");
        if (htmlElement) {
            resetDocumentTree(document, htmlElement);
            document.head = htmlElement.children.find((node) => node.tagName === "HEAD") ?? new Element("head");
            document.body = htmlElement.children.find((node) => node.tagName === "BODY") ?? new Element("body");

            if (document.head.parentNode !== document.documentElement) {
                document.documentElement.appendChild(document.head);
            }
            if (document.body.parentNode !== document.documentElement) {
                document.documentElement.appendChild(document.body);
            }
            return document;
        }

        clearChildren(document.body);
        for (const node of parsed.nodes) {
            document.body.appendChild(node);
        }
        return document;
    }

    function parseXmlDocument(source, mimeType) {
        const document = new Document();
        document.contentType = mimeType;
        const parsed = parseMarkup(document, source, true);

        if (parsed.error || parsed.nodes.filter((node) => node.nodeType === 1).length !== 1) {
            return createParserErrorDocument(document, mimeType);
        }

        const root = parsed.nodes.find((node) => node.nodeType === 1);
        clearChildren(document);
        document.documentElement = root;
        document.head = null;
        document.body = null;
        document.appendChild(root);
        return document;
    }

    function createParserErrorDocument(document, mimeType) {
        const errorDocument = new Document();
        errorDocument.contentType = mimeType;
        const parserError = new Element("parsererror");
        parserError.textContent = "Invalid XML";
        clearChildren(errorDocument);
        errorDocument.documentElement = parserError;
        errorDocument.head = null;
        errorDocument.body = null;
        errorDocument.appendChild(parserError);
        return errorDocument;
    }

    function resetDocumentTree(document, documentElement) {
        clearChildren(document);
        document.documentElement = documentElement;
        document.appendChild(documentElement);
    }

    function clearChildren(node) {
        node.childNodes = [];
    }

    function isRawTextTag(tagName) {
        const normalized = String(tagName ?? "").toUpperCase();
        return normalized === "SCRIPT" || normalized === "STYLE";
    }

    function findTagEnd(source, startIndex) {
        let quote = null;

        for (let index = startIndex; index < source.length; index += 1) {
            const character = source[index];
            if (quote) {
                if (character === quote) {
                    quote = null;
                }
                continue;
            }

            if (character === '"' || character === "'") {
                quote = character;
                continue;
            }

            if (character === ">") {
                return index;
            }
        }

        return -1;
    }

    function findRawTextClosingTag(source, tagName, startIndex) {
        const closingTag = `</${String(tagName ?? "").toLowerCase()}>`;
        return source.toLowerCase().indexOf(closingTag, startIndex);
    }

    function parseMarkup(document, source, strictXml) {
        const stack = [];
        const nodes = [];
        let index = 0;

        while (index < source.length) {
            if (source.startsWith("<!--", index)) {
                const commentEnd = source.indexOf("-->", index + 4);
                if (commentEnd < 0) {
                    return { error: true, nodes: [] };
                }
                index = commentEnd + 3;
                continue;
            }

            if (source.startsWith("<!", index)) {
                const declarationEnd = source.indexOf(">", index + 2);
                if (declarationEnd < 0) {
                    return { error: true, nodes: [] };
                }
                index = declarationEnd + 1;
                continue;
            }

            if (source[index] === "<") {
                if (source.startsWith("</", index)) {
                    const closeEnd = findTagEnd(source, index + 2);
                    if (closeEnd < 0) {
                        return { error: true, nodes: [] };
                    }

                    const tagName = source.slice(index + 2, closeEnd).trim().toUpperCase();
                    const current = stack.pop();
                    if (!current || current.tagName !== tagName) {
                        return { error: true, nodes: [] };
                    }
                    index = closeEnd + 1;
                    continue;
                }

                const openEnd = findTagEnd(source, index + 1);
                if (openEnd < 0) {
                    return { error: true, nodes: [] };
                }

                const rawContent = source.slice(index + 1, openEnd);
                const selfClosing = /\/\s*$/.test(rawContent);
                const content = rawContent.replace(/\/\s*$/, "").trim();
                const parts = content.split(/\s+/, 2);
                const tagName = parts[0];
                const element = document.createElement(tagName);
                for (const [name, value] of parseAttributes(content.slice(tagName.length))) {
                    element.setAttribute(name, value);
                }

                if (stack.length > 0) {
                    stack[stack.length - 1].appendChild(element);
                } else {
                    nodes.push(element);
                }

                index = openEnd + 1;

                if (selfClosing) {
                    continue;
                }

                stack.push(element);
                if (isRawTextTag(tagName)) {
                    const rawTextEnd = findRawTextClosingTag(source, tagName, index);
                    if (rawTextEnd < 0) {
                        return { error: true, nodes: [] };
                    }

                    const rawText = source.slice(index, rawTextEnd);
                    if (rawText) {
                        element.appendChild(document.createTextNode(rawText));
                    }

                    stack.pop();
                    index = rawTextEnd + String(tagName).length + 3;
                }
                continue;
            }

            const nextTag = source.indexOf("<", index);
            const text = nextTag < 0 ? source.slice(index) : source.slice(index, nextTag);

            if (!strictXml && stack.length === 0 && !text.trim()) {
                index = nextTag < 0 ? source.length : nextTag;
                continue;
            }

            const textNode = document.createTextNode(text);
            if (stack.length > 0) {
                stack[stack.length - 1].appendChild(textNode);
            } else {
                nodes.push(textNode);
            }

            index = nextTag < 0 ? source.length : nextTag;
        }

        if (strictXml && stack.length > 0) {
            return { error: true, nodes: [] };
        }

        return { error: false, nodes };
    }

    function parseAttributes(source) {
        const attributes = [];
        const pattern = /([^\s=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+)))?/g;
        let match = pattern.exec(source);

        while (match) {
            attributes.push([match[1], match[2] ?? match[3] ?? match[4] ?? ""]);
            match = pattern.exec(source);
        }

        return attributes;
    }
