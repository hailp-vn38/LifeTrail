import type { PlaybackValidationError } from "./timeline";
import type { PlaybackBreak } from "./types";

export const ISSUE_TEXT: Record<PlaybackValidationError, string> = {
  "not-linestring": "Dữ liệu route không hợp lệ nên không thể phát lại.",
  "too-few-coordinates": "Route chỉ có một điểm nên không thể phát lại.",
  "missing-timestamps": "Thiếu timestamps nên không thể phát lại theo thời gian GPS.",
  "timestamp-count-mismatch": "Timestamps không khớp số điểm GPS nên không thể phát lại.",
  "invalid-timestamp": "Timestamp không hợp lệ nên không thể phát lại.",
  "non-monotonic-timestamps": "Timestamps không theo thứ tự thời gian nên không thể phát lại.",
};

export const PART_ISSUE_TEXT = {
  "invalid-route-parts": "Route Parts đã xuất bản không hợp lệ nên không thể phát lại.",
  "too-few-observations": "Chưa đủ quan sát GPS để phát lại Route Parts.",
};

export const INTERRUPTION_TEXT: Record<PlaybackBreak, string> = {
  "gps-gap": "Thiếu GPS: đang giữ vị trí quan sát cuối cùng cho tới lần quan sát tiếp theo.",
  "evidence-hole": "Không đủ bằng chứng hoạt động: không suy diễn chuyển động qua khoảng này.",
  disconnected: "Các Route Part không liên tục: không nội suy giữa hai phần route.",
};
