"""Builds the anonymized synthetic .wfp fixture, shaped like Filmora 15 output.

    python3 make_synthetic.py synthetic.wfp text_script_template.json

All names, paths and ids are made up; ids are derived from labels so the
output is deterministic.
"""
import base64, hashlib, json, struct, sys, zipfile

OUT = sys.argv[1]
T = 10_000_000


def guid(label):
    h = hashlib.md5(label.encode()).hexdigest().upper()
    return '-'.join(h[i:i + 2] for i in range(0, 32, 2))


def uid(label):
    h = hashlib.md5(('uid' + label).encode()).hexdigest()
    return f'{h[:8]}-{h[8:12]}-4{h[13:16]}-a{h[17:20]}-{h[20:32]}'


def ud(key, data):
    if isinstance(data, int):
        raw = struct.pack('<i', data)
    elif isinstance(data, bytes):
        raw = data
    else:
        raw = data.encode()
    return {'data': base64.b64encode(raw).decode(), 'key': key, 'size': len(raw)}


def ud_guid(g):
    return ud(3, g.encode().ljust(64, b'\0'))


def kf(keys, total=None):
    h = hashlib.md5()
    parts = []
    for t, v, extra in keys:
        tt = float('%.8f' % t) if t else 0.0
        h.update(struct.pack('<ddd', tt, 6.0, v))
        s = '{"_time": %s,"Interpolation": 6,"_value": %r' % ('0.0' if t == 0 else '%.8f' % t, float(v))
        for k, d in extra:
            s += ',"%s": %s' % (k, d)
        parts.append(s + '}')
    out = '{"Version": 3,"ParameterType": 0,"keyframeSets": [%s],"MD5": "%s"' % (','.join(parts), h.hexdigest())
    if total is not None:
        out += ',"_totalTime": %s' % ('%.8f' % total)
    return out + '}'


EMPTY_KF = '{"Version": 3,"ParameterType": 0,"keyframeSets": [],"MD5":"d41d8cd98f00b204e9800998ecf8427e"}'
PIP = '{"Algorithm":"Bilinear","BlendMode":0,"BlenderName":"video/blender/simple-pip","Enable":true,"Opacity":%s}'


def fx(label, id_, display, params=(), extra=None):
    e = {'display': display, 'id': id_}
    if params:
        e['paramList'] = [{'fxParam': {'paramType': t, 'unValue': v}, 'name': n} for n, t, v in params]
    e.update({'placeholder': False, 'placeholderStatus': 0, 'thisUId': uid('fx' + label), 'type': 3,
              'userData': [ud(7, b'\x01'), ud_guid(guid('fx' + label))]})
    if extra:
        e.update(extra)
    return dict(sorted(e.items()))


def video_chains(label, transform=(), effect=(), color=()):
    chains = [{'effectList': [fx(label + 'crop', 'video/effect/crop-pan-zoom', 'crop-pan-zoom'),
                              fx(label + 'tf', 'video/effect/transform', 'transform',
                                 [('dwValue', 5, 1186339)] + list(transform) + [('EnableTransform', 5, 1)])],
               'name': 'Basic'}]
    if color:
        chains.append({'effectList': list(color), 'name': 'Color'})
    chains.append({'effectList': list(effect), 'name': 'Effect'})
    return chains


def audio_chains(label, gain=None, fade=None):
    vol = [('VolumeGain', 2, gain)] if gain is not None else []
    fd = [('FadeInTime', 2, fade[0]), ('FadeOutTime', 2, fade[1])] if fade else []
    return [{'effectList': [fx(label + 'cv', 'audio/effect/clip_volume', 'clip_volume'),
                            fx(label + 'cc', 'audio/effect/change_channel', 'change_channel'),
                            fx(label + 'vol', 'audio/effect/volume', 'volume', vol),
                            fx(label + 'duck', 'audio/effect/ducking', 'ducking'),
                            fx(label + 'fade', 'audio/effect/fade', 'fade', fd),
                            fx(label + 'eq', 'audio/effect/equalizer', 'equalizer')],
             'name': 'BasicFollow'}]


FILES = {
    'a': ('/Users/demo/Movies/Trip/CAM_0001.MP4', 'video', 60.06),
    'b': ('/Users/demo/Movies/Trip/CAM_0002.MP4', 'video', 30.03),
    'img': ('/Users/demo/Pictures/logo.png', 'image', 5.0),
    'music': ('/Users/demo/Music/theme.mp3', 'audio', 120.0),
}


def url(path):
    return 'file://' + path.lstrip('/')


def resource(k):
    path, kind, length = FILES[k]
    r = {'AIGCMeta': {'data': '', 'size': 0}, 'audioStreamCount': 0 if kind == 'image' else 1, 'bitRate': 0,
         'createDate': 0, 'filename': url(path), 'fourCC': 0, 'mediaLength': 0 if kind == 'image' else int(length * T),
         'modifyDate': 0, 'programCount': 1, 'sourceUuid': uid('src' + k),
         'streamType': {'video': 2, 'audio': 3, 'image': 5}[kind], 'subPicStreamCount': 0,
         'videoStreamCount': 0 if kind == 'audio' else 1}
    if kind != 'audio':
        r['vidStreamInfo'] = [{'frameRate': {'den': 100, 'num': 2997} if kind == 'video' else {'den': 1, 'num': 25},
                               'height': 1080 if kind == 'video' else 512, 'vidStreamId': 0,
                               'width': 1920 if kind == 'video' else 512}]
    return dict(sorted(r.items()))


def speed(offset, offset_end, rate, total):
    if rate == 1:
        keys = [(0, 1.0, []), (total, 1.0, [])]
    else:
        d = '%.8f' % ((1 - rate) / total)
        keys = [(0, float(rate), [('_rightDerivative', d)]), (total, 1.0, [('_leftDerivative', d)])]
    return {'offset': offset, 'offsetEnd': offset_end, 'reverse': False, 'speedParam': kf(keys, total)}


def media_clip(label, k, ty, tl_begin, tl_end, src_start, rate=1.0, link=None, chains=None, stream=0, opacity=100.0,
               extra=None):
    path, kind, length = FILES[k]
    dur = tl_end - tl_begin
    c = {'effectChainList': chains, 'filename': url(path), 'inPoint': round(src_start * T / rate),
         'outPoint': round(src_start * T / rate) + dur, 'placeholder': False, 'placeholderStatus': 0,
         'sourceUuid': uid('src' + k), 'streamId': stream, 'thisUId': uid(label), 'tlBegin': tl_begin,
         'tlEnd': tl_end, 'type': ty,
         'userData': [ud(10, guid('media' + k)), ud_guid(link or guid('link' + label)), ud(1, ty), ud(6, 9),
                      ud(50, path.rsplit('/', 1)[1].rsplit('.', 1)[0])]}
    if kind != 'image':
        c['speed'] = speed(src_start, src_start + dur / T * rate, rate, length)
    if ty == 1:
        c.update({'backgroundFillBluredness': 20, 'picSequence': False, 'pipBuf': PIP % repr(opacity),
                  'pipBufSize': len(PIP % repr(opacity)) + 1,
                  'stabilization': {'cache_mode': 0, 'smooth': 5.0, 'status': 0, 'version': 1.0}})
    else:
        c.update({'audioDuckingframe': {'parameter': EMPTY_KF}, 'denoiseV3Strength': 50.0,
                  'volumeKeyframe': {'parameter': EMPTY_KF}})
    if extra:
        c.update(extra)
    return dict(sorted(c.items()))


def transition(label, id_, display, name, res_id, begin, end):
    info = {'id': res_id, 'infoType': 7, 'is_custom': 0, 'res_type': 2, 'name': name, 'user_string': {}}
    return {'display': display, 'id': id_, 'placeholder': False, 'placeholderStatus': 0, 'thisUId': uid('tr' + label),
            'tlBegin': begin, 'tlEnd': end, 'type': 5,
            'userData': [ud(80, 1), ud_guid(guid('tr' + label)), ud(8, 3), ud(12, json.dumps(info, separators=(',', ':')))]}


DISSOLVE = '2981D185-D52E-44f4-ABD5-3CE83890E32E'
ADJUST = '662E16ED-4524-4D13-AAE9-11DBA0C63E17'

# Main timeline -----------------------------------------------------------------
s = T
la, lb = guid('pairA'), guid('pairB')
color = fx('adj', ADJUST, 'AdjustColor', [('bEnableLight', 5, 1), ('u_exposure', 3, 40.0)])
clip_a = media_clip('va', 'a', 1, 0, 5 * s, 2.0, link=la, chains=video_chains('va', color=[color]),
                    extra={'futureField': {'nested': [1, 2, 3], 'flag': True}})
clip_a['postTransition'] = transition('ab', DISSOLVE, 'Dissolve', 'Dissolve', 39146, 4 * s + s // 2, 5 * s + s // 2)
clip_a['preTransition'] = transition('pre', DISSOLVE, 'Dissolve', 'Dissolve', 39146, 0, s // 2)
clip_b = media_clip('vb', 'b', 1, 5 * s, 8 * s, 10.0, rate=2.0, link=lb, chains=video_chains('vb'))
aud_a = media_clip('aa', 'a', 2, 0, 5 * s, 2.0, link=la, chains=audio_chains('aa', gain=-3.0), stream=1)
aud_a['postTransition'] = transition('aab', 'audio/blender/transition-fade', 'audio fade', 'Fade', 39154,
                                     4 * s + s // 2, 5 * s + s // 2)
aud_b = media_clip('ab', 'b', 2, 5 * s, 8 * s, 10.0, rate=2.0, link=lb, chains=audio_chains('ab'), stream=1)
music = media_clip('mu', 'music', 2, 0, 9 * s, 0.0, chains=audio_chains('mu', gain=-12.5, fade=(1.0, 2.0)))
image = media_clip('im', 'img', 1, 2 * s, 6 * s, 0.0,
                   chains=video_chains('im', transform=[('Position_x', 3, 0.25), ('Position_y', 3, 0.75),
                                                        ('Scale_x', 3, 50.0), ('Scale_y', 3, 50.0)]), opacity=80.0)
image['inPoint'] = 36000000000
image['outPoint'] = 36000000000 + 4 * s

title_ref = {
    'effectChainList': video_chains('tr') + [{'effectList': [], 'name': '_wes_hold_effectchain_'}],
    'inPoint': 0, 'isAudio': 0, 'keyframeScaleMode': 1, 'outPoint': 2 * s,
    'pipBuf': PIP % '100.0', 'pipBufSize': len(PIP % '100.0') + 1, 'placeholder': False, 'placeholderStatus': 0,
    'thisUId': uid('titleref'), 'timelineId': 10, 'tlBegin': 6 * s, 'tlEnd': 8 * s, 'type': 7,
    'userData': [ud(6, 9), ud(50, 'Basic 1'), ud(10, 'Basic_1'), ud(1, 12), ud_guid(guid('titleref')),
                 ud(12, json.dumps({'id': 203568, 'infoType': 7, 'res_type': 1, 'name': 'Basic_1'}, separators=(',', ':'))),
                 ud(55, json.dumps({'key_timeline_resource_info': json.dumps({'name': 'Subtitle 9'}),
                                    'key_timeline_type': '5'}, separators=(',', ':')) + '\0')]}
graphic = {
    'effectChainList': video_chains('gr') + [{'effectList': [
        fx('grpath', 'graphic/effect/path', 'path'),
        fx('grstroke', 'graphic/effect/path_stroke', 'path_stroke', [('lineSize', 5, 6)])], 'name': 'graphic'}],
    'filename': '', 'inPoint': 0, 'outPoint': s, 'pathLinkList': {}, 'pipBuf': PIP % '100.0',
    'pipBufSize': len(PIP % '100.0') + 1, 'serviceType': 'pen', 'thisUId': uid('graphic'),
    'tlBegin': 8 * s, 'tlEnd': 9 * s, 'type': 26, 'userData': [ud_guid(guid('graphic')), ud(6, 9)]}

bus = uid('bus')
main = {
    'aspectRatioX': 16, 'aspectRatioY': 9, 'audioBusInfos': [{'balance': 0.5, 'busUid': bus, 'gain': 0.0}],
    'channels': 2, 'format': 'f32', 'frameRate': {'den': 1001, 'num': 30000}, 'hdrMode': 0, 'hdrMode_force_16bit': 0,
    'resolutionHeight': 1080, 'resolutionWidth': 1920, 'sampleRate': 44100, 'timelineId': 9,
    'trackInfos': [
        {'busUuids': [bus], 'clipList': [music], 'trackType': 2, 'userData': [ud(12000, 45)], 'uuid': uid('t0')},
        {'busUuids': [bus], 'clipList': [aud_a, aud_b], 'trackTag': 1, 'trackType': 2, 'userData': [ud(20, 2)],
         'uuid': uid('t1')},
        {'clipList': [clip_a, clip_b], 'trackTag': 2, 'trackType': 1, 'userData': [ud(12000, 70), ud(21, 1)],
         'uuid': uid('t2')},
        {'busUuids': [bus], 'clipList': [], 'trackTag': 3, 'trackType': 2, 'userData': [ud(20, 4)], 'uuid': uid('t3')},
        {'clipList': [image, title_ref, graphic], 'trackTag': 4, 'trackType': 1,
         'userData': [ud(12000, 32), ud(21, 1)], 'uuid': uid('t4')},
    ],
    'type': 0, 'userData': [ud(50, 'Untitle\0'), ud_guid(guid('main'))]}

# Title sub-timeline -------------------------------------------------------------
script = json.load(open(sys.argv[2]))
script.update({'PosX': 0.5, 'PosY': 0.85, 'ScaleX': 0.15, 'Text': 'Hello World'})
script['TextData'][0]['CharData'] = 'Hello World'
script['TextData'][0]['Basic']['TextColor'] = [{'Color': 0xFFCC00}]
script['TextData'][0]['Border']['Enable'] = 1
script['TextData'][0]['Border']['Color'] = 0
script['TextData'][0]['Border']['Size'] = 4
script_text = json.dumps(script, separators=(',', ':'))
text = {
    'effectChainList': video_chains('tx', transform=[('Scale_y', 3, 6.666666507720947), ('Position_y', 3, 0.85),
                                                     ('Scale_x', 3, 15.0)])[:1] +
                       [{'effectList': [], 'name': 'Effect'}, {'effectList': [], 'name': '_wes_hold_effectchain_'}],
    'inPoint': 0, 'outPoint': 2 * s, 'pipBuf': PIP % '100.0', 'pipBufSize': len(PIP % '100.0') + 1,
    'placeholder': False, 'placeholderStatus': 0, 'scriptBuf': script_text, 'scriptBufSize': len(script_text) + 1,
    'thisUId': uid('text'), 'tlBegin': 0, 'tlEnd': 2 * s, 'type': 4,
    'userData': [ud(1, 5), ud(50, 'Layer 01'), ud(6, 10), ud_guid(guid('text'))]}
sub = dict(main, timelineId=10, type=1, audioBusInfos=[{'balance': 0.5, 'busUid': uid('bus2'), 'gain': 0.0}],
           trackInfos=[{'clipList': [text], 'trackType': 1, 'uuid': uid('t2')}], userData=[ud_guid(guid('sub'))])

TL_MEDIA = guid('timeline')
doc = {'currentTimelineId': 9, 'productName': 'Filmora', 'projectName': 'TLB', 'projectVersion': '6,14,100,43',
       'resources': [resource(k) for k in FILES], 'serialNumber': 11, 'serializationVersion': '28',
       'timelineInfos': [main, sub]}

info = {
    'vbl_modify_version': '8.7.0.28', 'project_os_name': 'MacOS', 'project_file_name': 'Synthetic Trip',
    'project_file_suffix': '.wfp', 'project_editor_name': 'Filmora', 'project_editor_modify_version': '15.7.3.12221',
    'project_timeline_duration': 9 * s, 'project_timeline_framerate': [30000, 1001],
    'project_timeline_resolution': [1920, 1080], 'project_timeline_ratio': [16, 9], 'project_sample_rate': 44100,
    'timeline_mediaId': TL_MEDIA, 'project_guid': guid('project'),
    'user_data': [{'key': 1, 'data': '1'}, {'key': 2, 'data': ''}]}

items = {}
for k, (path, kind, length) in FILES.items():
    items[guid('media' + k)] = {'download_url': path, 'id': guid('media' + k),
                                'media_type': {'video': 8, 'image': 16, 'audio': 262144}[kind],
                                'media_length': int(length * T), 'name': path.rsplit('/', 1)[1].rsplit('.', 1)[0],
                                'mark_info_list': [{'mark_in': -1, 'mark_out': -1}]}
items[TL_MEDIA] = {'name': 'Untitle', 'download_url': '', 'id': TL_MEDIA, 'media_type': 1048576, 'duration': 9 * s}
medias = {'media_structure': {'visible': 'true', 'Folder': {'visible': 'true', 'media_item': [guid('media' + k) for k in FILES]},
                              'media_item': TL_MEDIA}, 'media_items': items}

with zipfile.ZipFile(OUT, 'w') as z:
    for k, (path, kind, length) in FILES.items():
        z.writestr(zipfile.ZipInfo(f'ProjectFolder/Medias/{guid("media" + k)}/media.json', (2026, 1, 2, 3, 4, 6)),
                   json.dumps({'version': '1.1.7', 'file_name': path}, separators=(',', ':')), zipfile.ZIP_STORED)
    z.writestr(zipfile.ZipInfo('ProjectFolder/Anon/AppData/MacOS/functionExtraData.json', (2026, 1, 2, 3, 4, 6)),
               '{"9":"2"}', zipfile.ZIP_STORED)
    for name, data in [(f'ProjectFolder/Medias/{TL_MEDIA}/timeline.wesproj', json.dumps(doc, separators=(',', ':'))),
                       ('ProjectFolder/Medias/medias_info.json', json.dumps(medias, separators=(',', ':'))),
                       ('ProjectFolder/project_info.json', json.dumps(info, indent=4))]:
        z.writestr(zipfile.ZipInfo(name, (2026, 1, 2, 3, 4, 6)), data, zipfile.ZIP_DEFLATED)
    z.writestr(zipfile.ZipInfo('ProjectFolder/Anon/Cover/thumb.fsthumb', (2026, 1, 2, 3, 4, 6)), b'\x00synthetic\x01' * 4,
               zipfile.ZIP_DEFLATED)
print('wrote', OUT)
