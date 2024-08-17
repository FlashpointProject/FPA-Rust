import { createRoot } from 'react-dom/client';
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { App } from './App';
import { GamePage, loader as gameLoader } from './pages/GamePage';
import { EditGamePage, loader as editGameLoader } from './pages/EditGamePage';
import { TagPage, loader as tagLoader } from './pages/TagPage';
import { PlatformPage, loader as platformLoader } from './pages/PlatformPage';

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        path: "game/:gameId",
        element: <GamePage />,
        loader: gameLoader,
      },
      {
        path: 'game/:gameId/edit',
        element: <EditGamePage />,
        loader: editGameLoader
      },
      {
        path: "tag/:tagId",
        element: <TagPage />,
        loader: tagLoader,
      },
      {
        path: "platform/:tagId",
        element: <PlatformPage />,
        loader: platformLoader,
      }
    ],
  },
]);

const root = createRoot(document.getElementById('root') as any);
root.render(
  <RouterProvider router={router} />
);