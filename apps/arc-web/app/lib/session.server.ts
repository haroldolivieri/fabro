import { redirect } from "react-router";
import { createSqliteSessionStorage } from "./session-storage.server";

interface SessionData {
  userUrl: string;
  githubId: number;
  githubNodeId: string;
  githubLogin: string;
  name: string;
  email: string;
  avatarUrl: string;
  accessToken: string;
}

function getSessionStorage() {
  const secret = process.env.SESSION_SECRET;
  if (!secret) {
    throw new Error("SESSION_SECRET is not set");
  }
  return createSqliteSessionStorage(secret);
}

export async function getSession(request: Request) {
  const storage = getSessionStorage();
  return storage.getSession(request.headers.get("Cookie"));
}

export async function commitSession(session: Awaited<ReturnType<typeof getSession>>) {
  const storage = getSessionStorage();
  return storage.commitSession(session);
}

export async function destroySession(session: Awaited<ReturnType<typeof getSession>>) {
  const storage = getSessionStorage();
  return storage.destroySession(session);
}

export async function getUser(request: Request) {
  const session = await getSession(request);
  const githubLogin = session.get("githubLogin");
  if (!githubLogin) return null;
  return {
    userUrl: session.get("userUrl") ?? "",
    githubLogin,
    name: session.get("name") ?? githubLogin,
    email: session.get("email") ?? "",
    avatarUrl: session.get("avatarUrl") ?? "",
  };
}

export async function requireUser(request: Request) {
  const user = await getUser(request);
  if (!user) throw redirect("/auth/login");
  return user;
}
