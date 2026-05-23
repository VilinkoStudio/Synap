import { exportDatabase } from '$lib/server/service';

export async function GET() {
  const bytes = await exportDatabase();

  return new Response(new Uint8Array(bytes), {
    headers: {
      'content-type': 'application/octet-stream',
      'content-disposition': 'attachment; filename="synap-web.redb"'
    }
  });
}
