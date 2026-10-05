import createClient from "openapi-fetch";
import type { paths } from "./generated/lifetrail-v1";

export const api = createClient<paths>({
  baseUrl: typeof window === "undefined" ? "http://localhost" : window.location.origin,
});
