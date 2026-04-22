#!/usr/bin/env node
/**
 * Unterm MCP Server (stdio)
 *
 * 将 Unterm 的 TCP JSON-RPC 接口桥接为 MCP stdio 协议，
 * 供 Claude Code 等 AI 工具调用。
 *
 * 提供的工具：
 * - screenshot: 截取屏幕截图并返回 base64 图片
 * - terminal_read: 读取所有终端 session 的文本内容
 */

const net = require('net');
const fs = require('fs');
const path = require('path');
const { exec } = require('child_process');

const UNTERM_HOST = '127.0.0.1';
const UNTERM_PORT = 19876;

// 读取 auth token
function getAuthToken() {
  try {
    const tokenPath = path.join(process.env.USERPROFILE || process.env.HOME, '.unterm', 'auth_token');
    return fs.readFileSync(tokenPath, 'utf-8').trim();
  } catch {
    return null;
  }
}

// 发送 JSON-RPC 请求到 Unterm
function sendRpc(method, params = {}) {
  return new Promise((resolve, reject) => {
    const client = new net.Socket();
    const token = getAuthToken();
    let buffer = '';

    client.connect(UNTERM_PORT, UNTERM_HOST, () => {
      // 先认证
      const authReq = JSON.stringify({ jsonrpc: '2.0', method: 'auth.login', params: { token }, id: 0 }) + '\n';
      client.write(authReq);
    });

    let authenticated = false;
    client.on('data', (data) => {
      buffer += data.toString();
      const lines = buffer.split('\n');
      buffer = lines.pop() || '';

      for (const line of lines) {
        if (!line.trim()) continue;
        try {
          const resp = JSON.parse(line);
          if (!authenticated) {
            authenticated = true;
            // 发送实际请求
            const rpcReq = JSON.stringify({ jsonrpc: '2.0', method, params, id: 1 }) + '\n';
            client.write(rpcReq);
          } else {
            client.destroy();
            if (resp.error) {
              reject(new Error(resp.error.message));
            } else {
              resolve(resp.result);
            }
          }
        } catch {}
      }
    });

    client.on('error', (err) => reject(err));
    setTimeout(() => { client.destroy(); reject(new Error('timeout')); }, 5000);
  });
}

// 使用 PowerShell GDI 截图
function captureScreenshot() {
  return new Promise((resolve, reject) => {
    const tempFile = path.join(process.env.TEMP || '/tmp', `unterm_screenshot_${Date.now()}.png`);
    const ps = `
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bitmap = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
$bitmap.Save('${tempFile.replace(/\\/g, '\\\\')}')
$graphics.Dispose()
$bitmap.Dispose()
`;
    exec(`powershell -ExecutionPolicy Bypass -Command "${ps.replace(/\n/g, '; ')}"`, (err) => {
      if (err) return reject(err);
      try {
        const data = fs.readFileSync(tempFile);
        fs.unlinkSync(tempFile);
        resolve(data.toString('base64'));
      } catch (e) {
        reject(e);
      }
    });
  });
}

// MCP stdio 协议处理
let inputBuffer = '';

process.stdin.setEncoding('utf-8');
process.stdin.on('data', (chunk) => {
  inputBuffer += chunk;
  processMessages();
});

function sendResponse(response) {
  const json = JSON.stringify(response);
  const msg = `Content-Length: ${Buffer.byteLength(json)}\r\n\r\n${json}`;
  process.stdout.write(msg);
}

function processMessages() {
  while (true) {
    const headerEnd = inputBuffer.indexOf('\r\n\r\n');
    if (headerEnd === -1) break;

    const header = inputBuffer.substring(0, headerEnd);
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) {
      inputBuffer = inputBuffer.substring(headerEnd + 4);
      continue;
    }

    const contentLength = parseInt(match[1]);
    const bodyStart = headerEnd + 4;
    if (inputBuffer.length < bodyStart + contentLength) break;

    const body = inputBuffer.substring(bodyStart, bodyStart + contentLength);
    inputBuffer = inputBuffer.substring(bodyStart + contentLength);

    try {
      const request = JSON.parse(body);
      handleRequest(request);
    } catch {}
  }
}

async function handleRequest(request) {
  const { method, id, params } = request;

  switch (method) {
    case 'initialize':
      sendResponse({
        jsonrpc: '2.0',
        id,
        result: {
          protocolVersion: '2024-11-05',
          capabilities: { tools: {} },
          serverInfo: { name: 'unterm-mcp', version: '0.1.0' },
        },
      });
      break;

    case 'notifications/initialized':
      // 无需响应
      break;

    case 'tools/list':
      sendResponse({
        jsonrpc: '2.0',
        id,
        result: {
          tools: [
            {
              name: 'screenshot',
              description: '截取当前屏幕截图，返回 PNG 图片。用于查看 Unterm 终端窗口或桌面状态。',
              inputSchema: { type: 'object', properties: {}, required: [] },
            },
            {
              name: 'terminal_read',
              description: '读取 Unterm 所有终端 session 的屏幕文本内容',
              inputSchema: { type: 'object', properties: {}, required: [] },
            },
          ],
        },
      });
      break;

    case 'tools/call':
      await handleToolCall(id, params);
      break;

    default:
      sendResponse({
        jsonrpc: '2.0',
        id,
        error: { code: -32601, message: `Method not found: ${method}` },
      });
  }
}

async function handleToolCall(id, params) {
  const { name } = params;

  try {
    switch (name) {
      case 'screenshot': {
        const base64 = await captureScreenshot();
        sendResponse({
          jsonrpc: '2.0',
          id,
          result: {
            content: [
              { type: 'image', data: base64, mimeType: 'image/png' },
            ],
          },
        });
        break;
      }

      case 'terminal_read': {
        try {
          const result = await sendRpc('capture.screen');
          sendResponse({
            jsonrpc: '2.0',
            id,
            result: {
              content: [{ type: 'text', text: JSON.stringify(result, null, 2) }],
            },
          });
        } catch (e) {
          sendResponse({
            jsonrpc: '2.0',
            id,
            result: {
              content: [{ type: 'text', text: `Unterm 未连接: ${e.message}` }],
            },
          });
        }
        break;
      }

      default:
        sendResponse({
          jsonrpc: '2.0',
          id,
          error: { code: -32602, message: `Unknown tool: ${name}` },
        });
    }
  } catch (e) {
    sendResponse({
      jsonrpc: '2.0',
      id,
      result: {
        content: [{ type: 'text', text: `Error: ${e.message}` }],
        isError: true,
      },
    });
  }
}
