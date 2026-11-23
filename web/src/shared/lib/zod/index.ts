import { z } from "zod";

// The page policy forbids eval, which zod probes for with new Function
z.config({ jitless: true });

export { z };
