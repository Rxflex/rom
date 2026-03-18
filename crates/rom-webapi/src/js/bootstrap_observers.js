    const mutationObservers = new Set();
    const resizeObservers = new Set();
    const intersectionObservers = new Set();

    class ObserverBase {
        constructor(callback) {
            this.callback = typeof callback === "function" ? callback : () => {};
            this.targets = [];
        }

        observe(target, options = {}) {
            const existing = this.targets.find((entry) => entry.target === target);
            if (existing) {
                existing.options = options;
                return;
            }
            this.targets.push({ target, options });
        }

        unobserve(target) {
            this.targets = this.targets.filter((entry) => entry.target !== target);
        }

        disconnect() {
            this.targets = [];
        }

        takeRecords() {
            return [];
        }
    }

    class MutationObserver extends ObserverBase {
        constructor(callback) {
            super(callback);
            this.__records = [];
            this.__scheduled = false;
            mutationObservers.add(this);
        }

        observe(target, options = {}) {
            super.observe(target, normalizeMutationObserverOptions(options));
        }

        disconnect() {
            super.disconnect();
            this.__records = [];
            this.__scheduled = false;
        }

        takeRecords() {
            const records = this.__records.slice();
            this.__records = [];
            return records;
        }

        __enqueue(record) {
            this.__records.push(record);
            if (this.__scheduled) {
                return;
            }
            this.__scheduled = true;
            queueMicrotask(() => {
                this.__scheduled = false;
                if (!this.targets.length || !this.__records.length) {
                    this.__records = [];
                    return;
                }
                this.callback(this.takeRecords(), this);
            });
        }
    }

    class ResizeObserver extends ObserverBase {
        constructor(callback) {
            super(callback);
            this.__entries = [];
            this.__scheduled = false;
            resizeObservers.add(this);
        }

        observe(target) {
            const existing = this.targets.find((entry) => entry.target === target);
            const rect = rectForTarget(target);
            if (existing) {
                existing.lastRect = rect;
            } else {
                this.targets.push({ target, lastRect: rect });
            }
            this.__enqueue(target, rect);
        }

        disconnect() {
            super.disconnect();
            this.__entries = [];
            this.__scheduled = false;
        }

        takeRecords() {
            const entries = this.__entries.slice();
            this.__entries = [];
            return entries;
        }

        __enqueue(target, rect) {
            this.__entries.push(createResizeObserverEntry(target, rect));
            if (this.__scheduled) {
                return;
            }
            this.__scheduled = true;
            queueMicrotask(() => {
                this.__scheduled = false;
                if (!this.targets.length || !this.__entries.length) {
                    this.__entries = [];
                    return;
                }
                this.callback(this.takeRecords(), this);
            });
        }
    }

    class IntersectionObserver extends ObserverBase {
        constructor(callback, options = {}) {
            super(callback);
            this.root = options.root ?? null;
            this.rootMargin = String(options.rootMargin ?? "0px");
            this.thresholds = normalizeThresholds(options.threshold);
            this.__entries = [];
            this.__scheduled = false;
            intersectionObservers.add(this);
        }

        observe(target) {
            if (!this.targets.some((entry) => entry.target === target)) {
                this.targets.push({ target, options: null });
            }
            this.__enqueue(target);
        }

        disconnect() {
            super.disconnect();
            this.__entries = [];
            this.__scheduled = false;
        }

        takeRecords() {
            const entries = this.__entries.slice();
            this.__entries = [];
            return entries;
        }

        __enqueue(target) {
            this.__entries.push(createIntersectionObserverEntry(target, this.root));
            if (this.__scheduled) {
                return;
            }
            this.__scheduled = true;
            queueMicrotask(() => {
                this.__scheduled = false;
                if (!this.targets.length || !this.__entries.length) {
                    this.__entries = [];
                    return;
                }
                this.callback(this.takeRecords(), this);
            });
        }
    }

    function normalizeMutationObserverOptions(options) {
        return {
            childList: Boolean(options.childList),
            attributes: Boolean(options.attributes),
            characterData: Boolean(options.characterData),
            subtree: Boolean(options.subtree),
            attributeOldValue: Boolean(options.attributeOldValue),
            characterDataOldValue: Boolean(options.characterDataOldValue),
            attributeFilter: Array.isArray(options.attributeFilter)
                ? options.attributeFilter.map(String)
                : null,
        };
    }

    function rectForTarget(target) {
        const rect = typeof target?.getBoundingClientRect === "function"
            ? target.getBoundingClientRect()
            : { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };
        return {
            x: rect.x ?? 0,
            y: rect.y ?? 0,
            width: rect.width ?? 0,
            height: rect.height ?? 0,
            top: rect.top ?? 0,
            left: rect.left ?? 0,
            right: rect.right ?? 0,
            bottom: rect.bottom ?? 0,
        };
    }

    function createResizeObserverEntry(target, rect) {
        return {
            target,
            contentRect: { ...rect },
            borderBoxSize: [{ inlineSize: rect.width, blockSize: rect.height }],
            contentBoxSize: [{ inlineSize: rect.width, blockSize: rect.height }],
            devicePixelContentBoxSize: [{ inlineSize: rect.width, blockSize: rect.height }],
        };
    }

    function createIntersectionObserverEntry(target, root) {
        const rect = rectForTarget(target);
        const isIntersecting = isConnectedToDocument(target);
        return {
            time: performance.now(),
            target,
            rootBounds: root ? rectForTarget(root) : null,
            boundingClientRect: { ...rect },
            intersectionRect: isIntersecting ? { ...rect } : emptyRect(),
            isIntersecting,
            intersectionRatio: isIntersecting ? 1 : 0,
        };
    }

    function emptyRect() {
        return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };
    }

    function normalizeThresholds(threshold) {
        if (Array.isArray(threshold)) {
            return threshold.map(Number).sort((left, right) => left - right);
        }
        if (threshold === undefined) {
            return [0];
        }
        return [Number(threshold)];
    }

    function isConnectedToDocument(target) {
        let current = target ?? null;
        while (current) {
            if (current.nodeType === 9) {
                return true;
            }
            current = current.parentNode ?? null;
        }
        return false;
    }

    function matchesObservedTarget(target, observedTarget, subtree) {
        if (target === observedTarget) {
            return true;
        }
        if (!subtree) {
            return false;
        }
        let current = target?.parentNode ?? null;
        while (current) {
            if (current === observedTarget) {
                return true;
            }
            current = current.parentNode;
        }
        return false;
    }

    function buildMutationRecord(record, oldValue) {
        return {
            type: record.type,
            target: record.target,
            addedNodes: record.addedNodes ?? [],
            removedNodes: record.removedNodes ?? [],
            previousSibling: record.previousSibling ?? null,
            nextSibling: record.nextSibling ?? null,
            attributeName: record.attributeName ?? null,
            oldValue,
        };
    }

    g.__rom_queueMutationRecord = (record) => {
        for (const observer of mutationObservers) {
            for (const entry of observer.targets) {
                if (!matchesObservedTarget(record.target, entry.target, entry.options.subtree)) {
                    continue;
                }

                if (record.type === "childList" && entry.options.childList) {
                    observer.__enqueue(buildMutationRecord(record, null));
                    break;
                }

                if (record.type === "attributes" && entry.options.attributes) {
                    if (
                        entry.options.attributeFilter &&
                        !entry.options.attributeFilter.includes(record.attributeName)
                    ) {
                        continue;
                    }
                    observer.__enqueue(
                        buildMutationRecord(
                            record,
                            entry.options.attributeOldValue ? record.oldValue ?? null : null,
                        ),
                    );
                    break;
                }

                if (record.type === "characterData" && entry.options.characterData) {
                    observer.__enqueue(
                        buildMutationRecord(
                            record,
                            entry.options.characterDataOldValue ? record.oldValue ?? null : null,
                        ),
                    );
                    break;
                }
            }
        }
    };

    g.__rom_queueLayoutObservation = (target) => {
        for (const observer of resizeObservers) {
            for (const entry of observer.targets) {
                if (!matchesObservedTarget(target, entry.target, true)) {
                    continue;
                }
                const rect = rectForTarget(entry.target);
                if (
                    rect.width !== entry.lastRect.width ||
                    rect.height !== entry.lastRect.height
                ) {
                    entry.lastRect = rect;
                    observer.__enqueue(entry.target, rect);
                }
            }
        }

        for (const observer of intersectionObservers) {
            for (const entry of observer.targets) {
                if (!matchesObservedTarget(target, entry.target, true)) {
                    continue;
                }
                observer.__enqueue(entry.target);
            }
        }
    };
