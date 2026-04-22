const fs = require('fs');
const path = require('path');

// 打包前删除 node_modules，避免 Tauri 报错
// 打包后执行 pnpm install 即可恢复
const nm = path.join(__dirname, 'frontend', 'node_modules');
if (fs.existsSync(nm)) {
  fs.rmSync(nm, { recursive: true });
  console.log('node_modules removed for build');
}
console.log('Frontend ready for bundling');
