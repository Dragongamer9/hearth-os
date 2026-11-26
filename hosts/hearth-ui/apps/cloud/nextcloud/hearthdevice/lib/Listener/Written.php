<?php
declare(strict_types=1);

namespace OCA\HearthDevice\Listener;

use OCP\EventDispatcher\Event;
use OCP\EventDispatcher\IEventListener;
use OCP\Files\Events\Node\NodeWrittenEvent;
use OCP\IDBConnection;
use OCP\ISession;
use OCP\IUserSession;

/**
 * A file was written: the device that wrote it is kept as Hearth's WebDAV property {https://hearthos.fun/ns}device
 * (Nextcloud's own property table, as a PROPPATCH would keep it). The device is the name of the app password the
 * client signed in with (a phone's or desktop client's, named when it was approved), or of the browser session.
 * Hearth Cloud's own uploads name their device themselves afterwards (a PROPPATCH), which wins.
 * @template-implements IEventListener<Event>
 */
class Written implements IEventListener {
    public const NAME = '{https://hearthos.fun/ns}device';

    public function __construct(private IDBConnection $db, private ISession $session, private IUserSession $users) {
    }

    public function handle(Event $event): void {
        if (!($event instanceof NodeWrittenEvent)) {
            return;
        }
        $user = $this->users->getUser();
        if ($user === null) {
            return;
        }
        $device = $this->device();
        if ($device === '') {
            return;
        }
        // "/<user>/files/<path>" → the DAV path Nextcloud keeps properties under: "files/<user>/<path>"
        $parts = explode('/', ltrim($event->getNode()->getPath(), '/'), 3);
        if (count($parts) < 3 || $parts[1] !== 'files') {
            return;
        }
        $uid = $user->getUID();
        $path = 'files/' . $uid . '/' . $parts[2];
        try {
            $qb = $this->db->getQueryBuilder();
            $qb->delete('properties')
                ->where($qb->expr()->eq('userid', $qb->createNamedParameter($uid)))
                ->andWhere($qb->expr()->eq('propertypath', $qb->createNamedParameter($path)))
                ->andWhere($qb->expr()->eq('propertyname', $qb->createNamedParameter(self::NAME)))
                ->executeStatement();
            $qb = $this->db->getQueryBuilder();
            $qb->insert('properties')->values([
                'userid' => $qb->createNamedParameter($uid),
                'propertypath' => $qb->createNamedParameter($path),
                'propertyname' => $qb->createNamedParameter(self::NAME),
                'propertyvalue' => $qb->createNamedParameter($device),
            ])->executeStatement();
        } catch (\Throwable $e) {
            // never in the way of the file itself
        }
    }

    /** The name of the app password (or session) this request signed in with, shortened to the device's own. */
    private function device(): string {
        try {
            $provider = \OC::$server->get(\OC\Authentication\Token\IProvider::class);
            $token = $this->session->exists('app_password') ? (string)$this->session->get('app_password') : $this->session->getId();
            $name = trim($provider->getToken($token)->getName());
        } catch (\Throwable $e) {
            return '';
        }
        return mb_substr($name, 0, 80);
    }
}
