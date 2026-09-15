"""First-party geometry/color capture fixture; exported through the real glTF bridge."""
import json
import bpy
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view

def generate(out):
    original=bpy.context.window.scene
    scene=bpy.data.scenes.new('CaptureCalibration');bpy.context.window.scene=scene
    scene.render.engine='BLENDER_EEVEE';scene.render.resolution_x=512;scene.render.resolution_y=512;scene.render.resolution_percentage=100
    scene.render.image_settings.file_format='PNG';scene.render.image_settings.color_mode='RGBA';scene.render.image_settings.color_depth='8'
    scene.render.use_compositing=False;scene.view_settings.view_transform='Standard';scene.view_settings.look='None';scene.view_settings.exposure=0;scene.view_settings.gamma=1
    world=bpy.data.worlds.new('CalibrationWorld');world.use_nodes=True;world.node_tree.nodes['Background'].inputs['Color'].default_value=(.12,.12,.12,1);scene.world=world
    d=bpy.data.cameras.new('CalibrationCamera');cam=bpy.data.objects.new('CalibrationCamera',d);scene.collection.objects.link(cam);scene.camera=cam
    cam.location=(0,-5,0);cam.rotation_euler=(Vector((0,0,0))-cam.location).to_track_quat('-Z','Y').to_euler();d.type='ORTHO';d.ortho_scale=3;d.clip_start=.01;d.clip_end=100
    markers=[]
    for i,(x,z,color) in enumerate([(-.9,.65,[1,0,0]),(.1,.45,[0,1,0]),(.85,.7,[0,0,1]),(-.8,-.65,[.03]*3),(0,-.4,[.18]*3),(.8,-.7,[.7]*3)]):
        mesh=bpy.data.meshes.new(f'Marker{i}');w=.16+i*.012
        mesh.from_pydata([(x-w,0,z-w),(x+w,0,z-w),(x+w,0,z+w),(x-w,0,z+w)],[],[(0,1,2,3)])
        obj=bpy.data.objects.new(mesh.name,mesh);scene.collection.objects.link(obj)
        mat=bpy.data.materials.new(mesh.name);mat.use_nodes=True;nt=mat.node_tree;nt.nodes.clear();em=nt.nodes.new('ShaderNodeEmission');em.inputs['Color'].default_value=(*color,1);nt.links.new(em.outputs[0],nt.nodes.new('ShaderNodeOutputMaterial').inputs['Surface']);mesh.materials.append(mat)
        bpy.context.view_layer.update();projected=world_to_camera_view(scene,cam,Vector((x,0,z)))
        markers.append({'name':mesh.name,'world_blender':[x,0,z],'world_gltf':[x,z,0],'pixel':[round(projected.x*512),round((1-projected.y)*512)],'linear_color':color})
    scene.render.filepath=str(out/'calibration-blender.png');bpy.ops.render.render(write_still=True)
    bpy.ops.export_scene.gltf(filepath=str(out/'runtime/calibration.glb'),export_format='GLB',use_active_scene=True,export_cameras=False,export_lights=False,export_yup=True,export_animations=False,will_save_settings=False)
    record={'camera':{'position':[0,0,5],'target':[0,0,0],'up':[0,1,0],'projection':'orthographic','scale':3,'near':.01,'far':100},'markers':markers,'tolerances':{'center_channel_delta':2,'screen_position_pixels':1},'purpose':'geometry axes, projection, linear to sRGB and exposure; not NPR shading parity'}
    (out/'calibration.json').write_text(json.dumps(record,indent=2)+'\n');bpy.context.window.scene=original
