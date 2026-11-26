<?php
declare(strict_types=1);

namespace OCA\HearthDevice\AppInfo;

use OCA\HearthDevice\Listener\Written;
use OCP\AppFramework\App;
use OCP\AppFramework\Bootstrap\IBootContext;
use OCP\AppFramework\Bootstrap\IBootstrap;
use OCP\AppFramework\Bootstrap\IRegistrationContext;
use OCP\Files\Events\Node\NodeWrittenEvent;

class Application extends App implements IBootstrap {
    public function __construct() {
        parent::__construct('hearthdevice');
    }

    public function register(IRegistrationContext $context): void {
        $context->registerEventListener(NodeWrittenEvent::class, Written::class);
    }

    public function boot(IBootContext $context): void {
    }
}
