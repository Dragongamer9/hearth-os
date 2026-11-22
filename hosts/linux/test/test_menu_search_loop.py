"""The menus' search results are a binding (MenuSearch.qml's result, Launcher.qml's hearthAnswers) that calls
Launcher.qml's answersFor. Anything there that starts a timer or process and reads its state made the binding watch it:
on the owner's dad's PC (Nov 21, 2026) the results were rebuilt 20 times a second and Quickshell crashed. SearchSources'
lookup() must only ever be called after the binding (Qt.callLater), never inside it."""
import os
import re
import unittest

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')


class MenuSearchLoop(unittest.TestCase):
    def test_lookup_is_deferred(self):
        src = open(os.path.join(SHELL, 'Launcher.qml'), encoding='utf-8').read()
        body = src[src.index('function answersFor('):]
        body = body[:body.index('\n    }\n')]
        self.assertIn('Qt.callLater(sources.lookup', body)
        self.assertIsNone(re.search(r'(?<!callLater\()sources\.lookup\(', body), 'sources.lookup() called inside the binding')


if __name__ == '__main__':
    unittest.main()
