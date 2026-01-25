"use client";

import * as React from "react";
import {
  CodeBlock as CodeBlockPrimitive,
  type CodeBlockProps as CodeBlockPropsPrimitive,
} from "@/components/primitives/animate/code-block";
import { cn } from "@/lib/utils";
import { getStrictContext } from "@/lib/get-strict-context";

type CodeContextType = {
  code: string;
};

const [CodeProvider, useCode] =
  getStrictContext<CodeContextType>("CodeContext");

type CodeProps = React.ComponentProps<"div"> & {
  code: string;
};

function Code({ className, code, ...props }: CodeProps) {
  return (
    <CodeProvider value={{ code }}>
      <div
        className={cn(
          "relative flex flex-col overflow-hidden border bg-accent/50 rounded-lg",
          className
        )}
        {...props}
      />
    </CodeProvider>
  );
}

type CodeBlockProps = Omit<CodeBlockPropsPrimitive, "code"> & {
  cursor?: boolean;
};

function CodeBlock({ cursor, className, ...props }: CodeBlockProps) {
  const { code } = useCode();
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const theme = localStorage.getItem("theme") || "light";

  return (
    <CodeBlockPrimitive
      ref={scrollRef}
      theme={theme === "dark" ? "dark" : "light"}
      scrollContainerRef={scrollRef}
      className={cn(
        "relative text-3xl overflow-auto",
        "[&>pre,_&_code]:!bg-transparent [&>pre,_&_code]:[background:transparent_!important] [&>pre,_&_code]:border-none [&_code]:!text-[13px] [&_code_.line]:!px-0",
        cursor &&
          "data-[done=false]:[&_.line:last-of-type::after]:content-['|'] data-[done=false]:[&_.line:last-of-type::after]:inline-block data-[done=false]:[&_.line:last-of-type::after]:w-[1ch] data-[done=false]:[&_.line:last-of-type::after]:-translate-px",
        className
      )}
      code={code}
      {...props}
    />
  );
}

export { Code, CodeBlock, type CodeProps, type CodeBlockProps };
