import { importDatabase } from '$lib/server/service';
import { redirect } from '@sveltejs/kit';

export async function POST({ request }) {
  const formData = await request.formData();
  const file = formData.get('database');

  if (!(file instanceof File)) {
    return new Response('database file is required', {
      status: 400
    });
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  console.info(`[synap-web] upload database name=${file.name} size=${file.size} bytes=${bytes.byteLength}`);

  try {
    await importDatabase(bytes);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return new Response(`database import failed: ${message}`, {
      status: 400
    });
  }

  throw redirect(303, '/');
}
