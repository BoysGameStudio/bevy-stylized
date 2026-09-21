import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
from project import Project
from assets.prepare import prepare, verify
from assets.synthetic import generate
from checks.dependencies import violations


class ProjectTools(unittest.TestCase):
    def test_compiler_cache_invalidates_on_gltf_contract_and_dependency_changes(self):
        from assets.build import compiler_inputs
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            (root/'Cargo.toml').write_text('[workspace]')
            for name in ['tools', 'profile-genshin', 'core', 'gltf']:
                crate = root/'crates'/f'bevy-stylized-{name}'
                (crate/'src').mkdir(parents=True)
                (crate/'Cargo.toml').write_text('[package]')
                (crate/'src/lib.rs').write_text('// initial')
            before = compiler_inputs(root)
            (root/'crates/bevy-stylized-gltf/src/lib.rs').write_text('// new metadata contract')
            after = compiler_inputs(root)
            self.assertNotEqual(before, after)
            (root/'Cargo.toml').write_text('[workspace.dependencies]')
            self.assertNotEqual(after, compiler_inputs(root))

    def test_config_paths_are_relative_to_config_not_cwd(self):
        with tempfile.TemporaryDirectory(prefix='project with spaces ') as tmp:
            root = pathlib.Path(tmp)
            (root/'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
            cfg=root/'config';cfg.mkdir();(cfg/'settings.toml').write_text('[paths]\ncache="../my cache"\n')
            p=Project(root, cfg/'settings.toml')
            self.assertEqual(p.cache,root/'my cache')

    def test_hash_failure_cannot_replace_prepared_pack(self):
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);src=root/'mirror';src.mkdir();(src/'model.fbx').write_bytes(b'good')
            m={'schema':'bevy_stylized.source/v1','id':'test','files':[{'path':'model.fbx','bytes':4,'sha256':hashlib.sha256(b'good').hexdigest()}]}
            path=root/'manifest.json';path.write_text(json.dumps(m));out=prepare(path,root/'cache',src)
            (src/'model.fbx').write_bytes(b'evil')
            self.assertEqual(prepare(path,root/'cache',src),out)
            with self.assertRaises(ValueError):prepare(path,root/'another-cache',src)
            self.assertEqual((out/'model.fbx').read_bytes(),b'good')
            (out/'model.fbx').write_bytes(b'evil')
            with self.assertRaises(ValueError):prepare(path,root/'cache',src)

    def test_manifest_rejects_escape(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(ValueError):verify(tmp,{'schema':'bevy_stylized.source/v1','id':'test','files':[{'path':'../outside','bytes':0,'sha256':'0'*64}]})

    def test_synthetic_rebuild_is_deterministic_and_contains_animation(self):
        import struct
        with tempfile.TemporaryDirectory() as tmp:
            a=generate(pathlib.Path(tmp)/'one').read_bytes();b=generate(pathlib.Path(tmp)/'two').read_bytes()
            self.assertEqual(a,b)
            n=struct.unpack_from('<I',a,12)[0];j=json.loads(a[20:20+n])
            self.assertEqual(len(j['skins'][0]['joints']),2)
            self.assertEqual({c['target']['path'] for c in j['animations'][0]['channels']},{'weights','rotation'})
            self.assertEqual(len(j['materials']),3)
            self.assertTrue((pathlib.Path(tmp)/'one/character.npr.json').is_file())

    def test_graph_rejects_build_edge_and_external_package(self):
        m={'packages':[{'id':'core','name':'bevy-stylized-core','source':None,'manifest_path':'/repo/core/Cargo.toml',
                       'dependencies':[{'name':'bad','kind':'build','path':'/outside'}]},
                      {'id':'bad','name':'bad','source':None,'manifest_path':'/outside/Cargo.toml','dependencies':[]}],
           'resolve':{'nodes':[{'id':'core','deps':[{'pkg':'bad','dep_kinds':[{'kind':'build'}]}]}]}}
        errors=violations(m,'/repo');self.assertTrue(any('closure' in e for e in errors));self.assertTrue(any('external' in e for e in errors))


class CaptureCaseValidation(unittest.TestCase):
    def test_case_rejects_wrong_version_and_incomplete_matrix(self):
        from assets.capture_case import validate,SCHEMA
        with tempfile.TemporaryDirectory() as tmp:
            p=pathlib.Path(tmp)/'case.json';p.write_text(json.dumps({'schema':'wrong'}))
            with self.assertRaisesRegex(ValueError,'schema'):validate(p)
            p.write_text(json.dumps({'schema':SCHEMA,'coordinates':'gltf-y-up-metres','resolution':[512,512],'required_cases':[],'cases':[]}))
            with self.assertRaisesRegex(ValueError,'required cases'):validate(p)

    def test_matched_case_rejects_proxy_diagnostics_and_mismatched_grid(self):
        from assets.capture_case import validate
        with tempfile.TemporaryDirectory() as tmp:
            p=pathlib.Path(tmp)/'case.json'
            data={'schema':'bevy_stylized.character-capture/v3','supersample':4,'diagnostics':{'policy':'base-color/v1'},'sampling':{'matched':{'policy':'matched-box/v1','grid':4}}}
            p.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError,'actual lighting factors'):validate(p)
            data['diagnostics']['policy']='lighting-factors/v1';data['sampling']['matched']['grid']=8
            p.write_text(json.dumps(data))
            with self.assertRaisesRegex(ValueError,'policy mismatch'):validate(p)

    def test_case_rejects_nonfinite_data_and_escaping_paths(self):
        from assets.capture_case import vector,local
        for v in [[0,0,float('nan')],[0,0,float('inf')],[0,0]]:
            with self.assertRaises(ValueError):vector(v)
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(ValueError):local(pathlib.Path(tmp),'../outside')

    def test_actual_camera_and_light_are_checked_independently(self):
        from assets.capture_case import verify_actual
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp)
            actual={'camera':{'matrix_world':[[1,0,0,0],[0,0,-1,-5],[0,1,0,0],[0,0,0,1]],'ortho_scale':3},'exposure_ev':0,'gamma':1,'view_transform':'Standard','resolution':[512,512],'body_direction_blender':[0,-1,0]}
            case={'actual':'actual.json','camera':{'position':[0,0,5],'target':[0,0,0],'up':[0,1,0],'scale':3},'direction_to_light':[0,0,1]}
            (root/'actual.json').write_text(json.dumps(actual));verify_actual(root,case)
            case['camera']['position'][0]=1
            with self.assertRaisesRegex(ValueError,'camera'):verify_actual(root,case)
            case['camera']['position'][0]=0;case['direction_to_light']=[0,0,-1]
            with self.assertRaisesRegex(ValueError,'light'):verify_actual(root,case)

class MaterialRampResolution(unittest.TestCase):
    def test_named_ramps_are_per_material_and_independent_of_inventory_order(self):
        from assets.material_sources import ramp_bindings
        with tempfile.TemporaryDirectory() as tmp:
            root=pathlib.Path(tmp);files=[]
            for name in ['Body','Body01']:
                path=root/(name+'.json');path.write_text(json.dumps({'m_SavedProperties':{'m_TexEnvs':{'_PackedShadowRampTex':{'m_Texture':{'Name':name+'_Ramp'}}}}}))
                (root/(name+'_Ramp.png')).write_bytes(name.encode())
                for filename in [name+'.json',name+'_Ramp.png']:files.append({'path':filename,'sha256':hashlib.sha256((root/filename).read_bytes()).hexdigest()})
            recipe={'material_roles':{'a':'body','b':'hair'},'material_sources':{'a':'Body.json','b':'Body01.json'}}
            a=ramp_bindings(root,{'files':files},recipe);b=ramp_bindings(root,{'files':list(reversed(files))},recipe)
            self.assertEqual(a,b);self.assertNotEqual(a['a']['texture_sha256'],a['b']['texture_sha256'])
            self.assertEqual(a['b']['texture'],'Body01_Ramp.png')
            with self.assertRaisesRegex(ValueError,'missing named ramp'):ramp_bindings(root,{'files':[f for f in files if f['path']!='Body01_Ramp.png']},recipe)


class IndependentDependencyGraph(unittest.TestCase):
    def test_android_outputs_do_not_change_identity_or_source_snapshot(self):
        from checks.isolated import source_snapshot
        for git_checkout in [False, True]:
            with self.subTest(git_checkout=git_checkout), tempfile.TemporaryDirectory() as tmp:
                root = pathlib.Path(tmp)/'source'
                root.mkdir()
                (root/'Cargo.toml').write_text('[workspace]\nmembers=[]\n')
                android = root/'apps/android-smoke/android'
                sources = {
                    'apps/android-smoke/src/lib.rs': '// app source',
                    'apps/android-smoke/android/app/build.gradle': '// build configuration',
                    'apps/android-smoke/android/app/src/main/AndroidManifest.xml': '<manifest/>',
                    'apps/android-smoke/android/gradle/wrapper/gradle-wrapper.jar': 'wrapper',
                    'apps/android-smoke/android/gradlew': '#!/bin/sh\n',
                    'fixtures/build/authoring.json': '{}',
                    'crates/example/src/build/input.rs': '// real source, not generated',
                }
                for name, data in sources.items():
                    path = root/name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(data)
                (android/'.gitignore').write_bytes(
                    (ROOT/'apps/android-smoke/android/.gitignore').read_bytes())
                if git_checkout:
                    subprocess.run(['git', 'init', '-q', str(root)], check=True)
                    subprocess.run(['git', '-C', str(root), 'add', '.'], check=True)
                project = Project(root)
                before = project.source_identity()
                clean_snapshot = source_snapshot(project, pathlib.Path(tmp)/'clean')
                generated = ['.gradle/cache.bin', '.kotlin/session', '.idea/workspace.xml',
                             'local.properties', 'build/report.json', 'app/build/output.apk']
                for name in generated:
                    path = android/name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text('generated output')
                self.assertEqual(project.source_identity(), before)
                destination = pathlib.Path(tmp)/'after-build'
                self.assertEqual(source_snapshot(project, destination), clean_snapshot)
                self.assertEqual(Project(destination).source_identity(), before)
                for name in generated:
                    self.assertFalse((destination/'apps/android-smoke/android'/name).exists())
                for name in sources:
                    self.assertEqual((destination/name).read_bytes(), (root/name).read_bytes())
                # Real changes must still invalidate the same identity.
                for name in ['apps/android-smoke/src/lib.rs',
                             'apps/android-smoke/android/app/build.gradle',
                             'fixtures/build/authoring.json']:
                    path = root/name
                    saved = path.read_bytes()
                    path.write_bytes(saved + b'\nchanged')
                    self.assertNotEqual(project.source_identity(), before)
                    path.write_bytes(saved)

    def test_snapshot_does_not_inherit_parent_patch_or_mutate_source_lock(self):
        from checks.dependencies import independent_check
        with tempfile.TemporaryDirectory() as tmp:
            parent = pathlib.Path(tmp)
            root = parent / 'product'
            (root / 'src').mkdir(parents=True)
            (root / 'Cargo.toml').write_text('[workspace]\n[package]\nname="fixture"\nversion="0.1.0"\nedition="2021"\n')
            (root / 'src/lib.rs').write_text('')
            (parent / '.cargo').mkdir()
            # Cargo would reject this missing enclosing override in the original cwd.
            (parent / '.cargo/config.toml').write_text('[patch.crates-io]\nmissing={path="does-not-exist"}\n')
            project = Project(root)
            result = independent_check(project)
            self.assertEqual(result['errors'], [])
            self.assertEqual(result['mode'], 'independent-source-snapshot')
            self.assertFalse((root / 'Cargo.lock').exists())

    def test_product_cannot_depend_on_a_local_lab(self):
        m = {'packages': [
            {'id':'core','name':'bevy-stylized-core','source':None,
             'manifest_path':'/repo/crates/core/Cargo.toml',
             'dependencies':[{'name':'lab','kind':None,'path':'/repo/labs/lab'}]},
            {'id':'lab','name':'lab','source':None,
             'manifest_path':'/repo/labs/lab/Cargo.toml','dependencies':[]}],
            'resolve':{'nodes':[{'id':'core','deps':[{'pkg':'lab','dep_kinds':[{'kind':None}]}]}]}}
        self.assertTrue(any('forbidden product edge' in e for e in violations(m, '/repo')))
        self.assertTrue(any('closure' in e for e in violations(m, '/repo')))
