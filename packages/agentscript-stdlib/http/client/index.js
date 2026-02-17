function buildInit(options) {
  const init = {};
  if (options?.headers) {
    init.headers = { ...options.headers };
  }
  if (options?.timeout) {
    init.signal = AbortSignal.timeout(options.timeout);
  }
  if (options?.body != null) {
    if (typeof options.body === "string") {
      init.body = options.body;
      if (!init.headers?.["Content-Type"]) {
        init.headers = { "Content-Type": "text/plain", ...init.headers };
      }
    } else if (typeof options.body === "object") {
      init.body = JSON.stringify(options.body);
      if (!init.headers?.["Content-Type"]) {
        init.headers = { "Content-Type": "application/json", ...init.headers };
      }
    }
  }
  return init;
}

export async function get(url, options) {
  return fetch(url, { method: "GET", ...buildInit(options) });
}

export async function post(url, options) {
  return fetch(url, { method: "POST", ...buildInit(options) });
}

export async function put(url, options) {
  return fetch(url, { method: "PUT", ...buildInit(options) });
}

export async function del(url, options) {
  return fetch(url, { method: "DELETE", ...buildInit(options) });
}

export async function patch(url, options) {
  return fetch(url, { method: "PATCH", ...buildInit(options) });
}

export async function head(url, options) {
  return fetch(url, { method: "HEAD", ...buildInit(options) });
}

export async function options(url, options) {
  return fetch(url, { method: "OPTIONS", ...buildInit(options) });
}
