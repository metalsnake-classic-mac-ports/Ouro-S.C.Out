// The title picture appears the way the original shows its pictures
// (CODE 8 0x1416): in blocks of 16 x 14 pixels in random order, all 832
// of the 512 x 364 picture area. Without motion when the reader asks for it.
(function () {
  const canvas = document.getElementById("title");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const picture = new Image();
  picture.src = "site/title.png";
  picture.onload = function () {
    const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (still) {
      ctx.drawImage(picture, 0, 0);
      return;
    }
    const blocks = [];
    for (let y = 0; y < 26; y++) for (let x = 0; x < 32; x++) blocks.push([x * 16, y * 14]);
    for (let i = blocks.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [blocks[i], blocks[j]] = [blocks[j], blocks[i]];
    }
    const perFrame = 12; // about 70 frames, a bit over a second
    let next = 0;
    function step() {
      for (let k = 0; k < perFrame && next < blocks.length; k++, next++) {
        const [x, y] = blocks[next];
        ctx.drawImage(picture, x, y, 16, 14, x, y, 16, 14);
      }
      if (next < blocks.length) requestAnimationFrame(step);
    }
    requestAnimationFrame(step);
  };
})();
