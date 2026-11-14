"""The Recovery USB is made from Hearth OS on Debian (1.12): the devices' own kernel, drivers and boot chain, its own
boot menu tested in a virtual PC with Secure Boot on before anything is uploaded."""
import os
import unittest

R = os.path.join(os.path.dirname(__file__), '..', '..', '..')


def text(p):
    return open(os.path.join(R, p)).read()


class DebianRecovery(unittest.TestCase):
    def test_made_from_the_debian_image(self):
        c = text('os/recovery/Containerfile')
        self.assertIn('ARG BASE=ghcr.io/dragongamer9/hearth-peppermint:testing', c)
        w = text('.github/workflows/recovery-iso.yml')
        self.assertIn('IMAGE: ghcr.io/dragongamer9/hearth-peppermint', w)
        self.assertNotIn('ghcr.io/dragongamer9/hearth-os\n', w)

    def test_live_boot_and_signed_boot_chain(self):
        b = text('os/recovery/src/build.sh')
        self.assertNotIn('dnf ', b)
        self.assertIn('dracut-live', b)
        self.assertIn('cp -v /boot/efi/EFI/debian/shimx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI', b)
        self.assertIn('rm -f /boot/efi/EFI/BOOT/fbx64.efi', b)          # no boot entry written for the stick
        y = text('os/recovery/src/iso.yaml')
        self.assertIn('root=live:CDLABEL=HearthRecov', y)
        self.assertNotIn('enforcing=0', y)                                 # no SELinux on Debian
        self.assertIn('timeout: 3', y)

    def test_no_hearth_session_and_nothing_written_in_the_background(self):
        b = text('os/recovery/src/build.sh')
        for s in ('nimbus-autologin.service', 'hearth-recovery.service', 'hearth-bls-grub.service', 'bootc-fetch-apply-updates.timer',
                  'hearth-snapshots.timer', 'hearth-diskguard.service'):
            self.assertIn(s, b)

    def test_upload_only_after_the_boot_test(self):
        w = text('.github/workflows/recovery-iso.yml')
        self.assertIn("inputs.upload && steps.boot.outcome == 'success'", w)
        t = text('tools/recovery-boot-test.sh')
        self.assertIn('OVMF_CODE_4M.secboot.fd', t)
        self.assertIn('HEARTH-CI', t)


if __name__ == '__main__':
    unittest.main()
