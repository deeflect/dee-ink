"use client";

import { useRef, useEffect } from "react";

export default function InkCharacter() {
  const v1Ref = useRef<HTMLVideoElement>(null);
  const v2Ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const v1 = v1Ref.current;
    const v2 = v2Ref.current;
    if (!v1 || !v2) return;

    const FADE = 0.6;
    let active = v1;
    let standby = v2;

    const swap = () => {
      // Start standby from beginning, fade it in
      standby.currentTime = 0;
      standby.play();
      standby.style.opacity = "1";
      // Fade out active
      active.style.opacity = "0";

      const tmp = active;
      active = standby;
      standby = tmp;
    };

    const check = () => {
      if (active.duration && active.duration - active.currentTime < FADE) {
        swap();
      }
      requestAnimationFrame(check);
    };

    v1.play();
    requestAnimationFrame(check);
  }, []);

  const videoClass = "absolute inset-0 w-full h-full mix-blend-multiply transition-opacity duration-500";

  return (
    <div className="flex items-center justify-center">
      <div className="relative w-[260px] bg-white rounded-2xl overflow-hidden" style={{ aspectRatio: "1/1.3" }}>
        <video ref={v1Ref} className={videoClass} muted playsInline style={{ opacity: 1 }}>
          <source src="/character.webm" type="video/webm" />
        </video>
        <video ref={v2Ref} className={videoClass} muted playsInline style={{ opacity: 0 }}>
          <source src="/character.webm" type="video/webm" />
        </video>
      </div>
    </div>
  );
}
