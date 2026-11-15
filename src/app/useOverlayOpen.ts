import { useEffect, useState } from "react";

/** Radix 메뉴·다이얼로그는 body 바로 아래에 포털로 붙는다. 그것이 하나라도 떠 있는가. */
const overlayOpen = () =>
  !!document.querySelector("body > [data-radix-popper-content-wrapper], body > [role='dialog'], body > [role='menu'], body > .overlay");

/**
 * 메뉴나 다이얼로그가 떠 있는 동안 true. 인터넷 탭의 자식 웹뷰는 DOM 위에 겹쳐 그려져 이것들을 가리므로 그동안 숨긴다.
 * body 의 직접 자식만 지켜보므로 대화가 흘러도 비용이 거의 없다.
 */
export function useOverlayOpen() {
  const [open, setOpen] = useState(overlayOpen);
  useEffect(() => {
    const observer = new MutationObserver(() => setOpen(overlayOpen()));
    observer.observe(document.body, { childList: true });
    setOpen(overlayOpen());
    return () => observer.disconnect();
  }, []);
  return open;
}
