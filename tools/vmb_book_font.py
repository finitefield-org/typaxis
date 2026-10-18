"""Independent TrueType source joins for visible numbers and authored body text.

This reader checks original DejaVu/IPA source glyphs and simple-glyph subsets.
It does not extend public admission or establish original full-book font coverage.
"""
import hashlib
import re
import struct
from fractions import Fraction

from vmb_book_inputs import require


def checksum(raw):
    data = raw+b'\0'*((-len(raw))%4)
    return sum(struct.unpack('>'+str(len(data)//4)+'I',data)) & 0xffffffff


class TrueType:
    def __init__(self, raw):
        self.raw,self.tables,self.glyph_cache = raw,{},{}
        require(isinstance(raw,bytes) and len(raw) >= 12 and raw[:4] == b'\0\1\0\0', 'TrueType sfnt header')
        count = self.u16(raw,4)
        require(0 < count <= 256 and 12+count*16 <= len(raw), 'bounded TrueType table directory')
        intervals = []
        for ordinal in range(count):
            tag,expected,start,length = struct.unpack_from('>4sIII',raw,12+ordinal*16)
            require(tag not in self.tables and start >= 12+count*16 and start%4 == 0 and start+length <= len(raw),
                    'unique contained TrueType table')
            data = raw[start:start+length]
            checked = data[:8]+b'\0'*4+data[12:] if tag == b'head' else data
            require(checksum(checked) == expected, 'TrueType source table checksum '+repr(tag))
            self.tables[tag] = data
            if length:intervals.append((start,start+length))
        require(all(a[1] <= b[0] for a,b in zip(sorted(intervals),sorted(intervals)[1:])), 'nonoverlapping TrueType tables')
        require(checksum(raw) == 0xb1b0afba, 'TrueType whole-font checksum')
        require({b'head',b'hhea',b'maxp',b'hmtx',b'loca',b'glyf'} <= set(self.tables), 'TrueType outline/metric tables')
        head,hhea,maxp = (self.tables[tag] for tag in (b'head',b'hhea',b'maxp'))
        require(len(head) == 54 and len(hhea) == 36 and len(maxp) == 32
                and head[12:16] == b'_\x0f<\xf5', 'TrueType metric headers')
        self.units,self.glyph_count,self.metric_count = self.u16(head,18),self.u16(maxp,4),self.u16(hhea,34)
        self.loca_format = self.i16(head,50)
        require(16 <= self.units <= 16384 and 0 < self.metric_count <= self.glyph_count and self.loca_format in (0,1),
                'TrueType units/glyph/metric/index ranges')
        require(len(self.tables[b'loca']) == (self.glyph_count+1)*(4 if self.loca_format else 2)
                and len(self.tables[b'hmtx']) == 4*self.metric_count+2*(self.glyph_count-self.metric_count),
                'complete TrueType glyph indices and horizontal metrics')
        self.ascender,self.descender,self.line_gap = struct.unpack_from('>hhh',hhea,4)
        self.bbox = list(struct.unpack_from('>hhhh',head,36))

    @staticmethod
    def u16(raw, offset):
        require(0 <= offset <= len(raw)-2, 'contained TrueType uint16')
        return struct.unpack_from('>H',raw,offset)[0]

    @staticmethod
    def i16(raw, offset):
        require(0 <= offset <= len(raw)-2, 'contained TrueType int16')
        return struct.unpack_from('>h',raw,offset)[0]

    @staticmethod
    def u32(raw, offset):
        require(0 <= offset <= len(raw)-4, 'contained TrueType uint32')
        return struct.unpack_from('>I',raw,offset)[0]

    def metrics(self, gid):
        require(type(gid) is int and 0 <= gid < self.glyph_count, 'typed contained TrueType metric glyph')
        data = self.tables[b'hmtx']
        if gid < self.metric_count:return self.u16(data,gid*4),self.i16(data,gid*4+2)
        return self.u16(data,(self.metric_count-1)*4),self.i16(data,self.metric_count*4+(gid-self.metric_count)*2)

    def glyph_bytes(self, gid):
        require(type(gid) is int and 0 <= gid < self.glyph_count, 'typed contained TrueType outline glyph')
        data = self.tables[b'loca']
        if self.loca_format:start,end = self.u32(data,gid*4),self.u32(data,(gid+1)*4)
        else:start,end = self.u16(data,gid*2)*2,self.u16(data,(gid+1)*2)*2
        require(0 <= start <= end <= len(self.tables[b'glyf']), 'contained TrueType glyph program')
        return self.tables[b'glyf'][start:end]

    def glyph(self, gid):
        if gid in self.glyph_cache and type(gid) is int:return self.glyph_cache[gid]
        raw = self.glyph_bytes(gid)
        require(len(raw) >= 12, 'visible simple TrueType glyph')
        contours = self.i16(raw,0)
        require(0 < contours <= 256, 'chapter number uses nonempty simple contours')
        bbox = list(struct.unpack_from('>hhhh',raw,2))
        ends = [self.u16(raw,10+2*index) for index in range(contours)]
        require(all(a < b for a,b in zip(ends,ends[1:])) and ends[-1] < 65535, 'ordered TrueType contour ends')
        count = ends[-1]+1
        offset = 10+2*contours
        instructions = self.u16(raw,offset);offset += 2
        require(offset+instructions <= len(raw), 'contained TrueType glyph instructions')
        instruction_sha = hashlib.sha256(raw[offset:offset+instructions]).hexdigest();offset += instructions
        flags = []
        while len(flags) < count:
            require(offset < len(raw), 'complete TrueType point flags')
            flag = raw[offset];offset += 1
            require(flag & 0xc0 == 0, 'simple TrueType point flag vocabulary')
            repeat = 1
            if flag & 8:
                require(offset < len(raw), 'TrueType point flag repeat')
                repeat += raw[offset];offset += 1
            require(len(flags)+repeat <= count, 'bounded TrueType point flag repeat')
            flags.extend([flag]*repeat)
        axes = []
        for short,same in ((2,16),(4,32)):
            values,cursor = [],0
            for flag in flags:
                if flag & short:
                    require(offset < len(raw), 'complete short TrueType coordinate')
                    delta = raw[offset]*(1 if flag & same else -1);offset += 1
                elif flag & same:delta = 0
                else:delta = self.i16(raw,offset);offset += 2
                cursor += delta;values.append(cursor)
            axes.append(values)
        require(len(raw)-offset <= 3 and not any(raw[offset:]), 'zero TrueType glyph alignment padding')
        points = [[x,y,bool(flag & 1)] for x,y,flag in zip(*axes,flags)]
        require(min(x for x,_,_ in points) == bbox[0] and min(y for _,y,_ in points) == bbox[1]
                and max(x for x,_,_ in points) == bbox[2] and max(y for _,y,_ in points) == bbox[3],
                'original TrueType glyph bounds cover the actual points')
        model = {'contours':contours,'bbox':bbox,'end_points':ends,'points':points,
            'instruction_bytes':instructions,'instruction_sha256':instruction_sha,
            'program_bytes':offset,'program_sha256':hashlib.sha256(raw[:offset]).hexdigest()}
        self.glyph_cache[gid] = model
        return model

    def codepoint(self, codepoint):
        require(type(codepoint) is int and 0 <= codepoint <= 0x10ffff, 'typed Unicode codepoint')
        data = self.tables[b'cmap'];count = self.u16(data,2)
        require(4+8*count <= len(data), 'complete Unicode cmap records')
        results,seen = [],set()
        for index in range(count):
            platform,encoding,start = struct.unpack_from('>HHI',data,4+8*index)
            if platform != 0 and not (platform == 3 and encoding in (1,10)):continue
            if start in seen:continue
            seen.add(start);fmt = self.u16(data,start)
            if fmt == 12:
                length,groups = self.u32(data,start+4),self.u32(data,start+12)
                require(length == 16+12*groups and start+length <= len(data), 'complete format-12 Unicode cmap')
                previous,gid = -1,0
                for group in range(groups):
                    low,high,first = struct.unpack_from('>III',data,start+16+12*group)
                    require(previous < low <= high <= 0x10ffff, 'ordered format-12 Unicode ranges')
                    previous = high
                    if low <= codepoint <= high:gid = first+codepoint-low
                results.append(gid)
            elif fmt == 4 and codepoint <= 0xffff:
                length,segments = self.u16(data,start+2),self.u16(data,start+6)//2
                require(segments > 0 and 16+segments*8 <= length and start+length <= len(data), 'complete format-4 Unicode cmap')
                previous,gid = -1,0
                for segment in range(segments):
                    high = self.u16(data,start+14+segment*2)
                    low = self.u16(data,start+16+segments*2+segment*2)
                    require(previous < high and low <= high, 'ordered format-4 Unicode ranges')
                    previous = high
                    if low <= codepoint <= high:
                        delta = self.i16(data,start+16+segments*4+segment*2)
                        position = start+16+segments*6+segment*2
                        relative = self.u16(data,position)
                        if relative:
                            address = position+relative+2*(codepoint-low)
                            require(address+2 <= start+length, 'contained format-4 glyph index')
                            gid = self.u16(data,address)
                            if gid:gid = (gid+delta)&0xffff
                        else:gid = (codepoint+delta)&0xffff
                results.append(gid)
        require(results and len(set(results)) == 1 and 0 < results[0] < self.glyph_count,
                'consistent nonmissing source Unicode glyph')
        return results[0]


CMAP_HEAD = (b'/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n'
    b'/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n'
    b'/CMapName /Typaxis-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n')
CMAP_END = b'endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n'


def unicode_map(raw):
    require(raw.startswith(CMAP_HEAD) and raw.endswith(CMAP_END), 'chapter ToUnicode identity CMap header')
    body = raw[len(CMAP_HEAD):-len(CMAP_END)]
    result,offset = {},0
    for match in re.finditer(rb'([1-9][0-9]*) beginbfchar\n(.*?)endbfchar\n',body,re.S):
        require(match.start() == offset, 'complete chapter ToUnicode CMap blocks')
        rows = match[2].splitlines();require(len(rows) == int(match[1]), 'chapter CMap declared row count')
        for row in rows:
            pair = re.fullmatch(rb'<([0-9A-F]{4})> <([0-9A-F]{4})>',row)
            require(pair is not None, 'chapter CMap single scalar row')
            cid,value = int(pair[1],16),bytes.fromhex(pair[2].decode()).decode('utf-16-be')
            require(cid > 0 and cid not in result, 'unique nonzero chapter CID')
            result[cid] = value
        offset = match.end()
    require(offset == len(body) and result, 'complete nonempty chapter CMap')
    return result


def pdf_integer(value):
    from pypdf.generic import NumberObject
    require(type(value) in (int,NumberObject), 'exact PDF font integer')
    return int(value)


def pdf_widths(values):
    from pypdf.generic import ArrayObject
    result,offset = {},0
    require(isinstance(values,ArrayObject), 'PDF font widths array')
    while offset < len(values):
        low = pdf_integer(values[offset]);offset += 1
        require(offset < len(values), 'complete PDF font widths entry')
        if isinstance(values[offset],ArrayObject):
            widths = list(map(pdf_integer,values[offset]));offset += 1
        else:
            high = pdf_integer(values[offset]);offset += 1
            require(low <= high <= 65535 and offset < len(values), 'PDF font widths range')
            widths = [pdf_integer(values[offset])]*(high-low+1);offset += 1
        require(0 < low <= 65535 and widths and low+len(widths)-1 <= 65535, 'contained PDF CID widths')
        for cid,width in enumerate(widths,low):
            require(cid not in result and width > 0, 'unique positive PDF CID width')
            result[cid] = width
    return result


def verify_number_font(font, source, characters):
    """Join CIDs, Unicode, original glyph programs and source advances."""
    return _verify_font(font, source, characters, allow_space=False)


def verify_text_font(font, source, characters):
    """Join original simple glyphs, including an explicitly nonpainting U+0020."""
    return _verify_font(font, source, characters, allow_space=True)


def _verify_font(font, source, characters, *, allow_space):
    require(font['/Type'] == '/Font' and font['/Subtype'] == '/Type0' and font['/Encoding'] == '/Identity-H'
            and len(font['/DescendantFonts']) == 1, 'chapter number Type0 font')
    cid = font['/DescendantFonts'][0].get_object();descriptor = cid['/FontDescriptor']
    require(cid['/Type'] == '/Font' and cid['/Subtype'] == '/CIDFontType2'
            and '/FontFile2' in descriptor and '/FontFile3' not in descriptor, 'embedded chapter TrueType font')
    subset = TrueType(descriptor['/FontFile2'].get_data())
    mapping = unicode_map(font['/ToUnicode'].get_data())
    require(set(mapping.values()) == set(characters) and len(mapping) == len(set(characters))
            and subset.units == source.units and subset.glyph_count == len(mapping)+1, 'complete source number glyph set')
    widths = pdf_widths(cid['/W'])
    require(set(widths) == set(mapping), 'all source CIDs have explicit widths')
    index = cid['/CIDToGIDMap']
    if index == '/Identity':gids = list(range(subset.glyph_count))
    else:
        raw = index.get_data()
        require(len(raw)%2 == 0, 'complete two-byte PDF glyph map')
        gids = list(struct.unpack('>'+str(len(raw)//2)+'H',raw))
    require(len(gids) == max(mapping)+1 and gids[0] == 0 and set(gids) == set(range(subset.glyph_count)),
            'complete bijective chapter CID to subset glyph map')
    require(subset.glyph(0) == source.glyph(0) and subset.metrics(0) == source.metrics(0), 'original .notdef glyph retained')
    rows = []
    for owner,char in sorted(mapping.items()):
        original,target = source.codepoint(ord(char)),gids[owner]
        if char == ' ' and allow_space:
            require(source.glyph_bytes(original) == subset.glyph_bytes(target) == b'',
                    'authored ASCII space has no source or subset outline')
            outline = {'program_sha256':hashlib.sha256(b'').hexdigest()}
        else:
            outline = source.glyph(original)
            require(subset.glyph(target) == outline, 'original chapter number outline, hints and horizontal metrics')
        require(subset.metrics(target) == source.metrics(original),
                'original chapter number outline, hints and horizontal metrics')
        advance,lsb = source.metrics(original)
        require(widths[owner] == (advance*1000+source.units//2)//source.units,
                'PDF CID width from original source hmtx')
        rows.append({'cid':owner,'unicode':char,'source_gid':original,'subset_gid':target,
            'advance_units':advance,'left_side_bearing_units':lsb,'outline_program_sha256':outline['program_sha256']})
    scaled = lambda value:int(Fraction(value*1000,source.units))
    require(list(map(pdf_integer,descriptor['/FontBBox'])) == list(map(scaled,source.bbox))
            and pdf_integer(descriptor['/Ascent']) == scaled(source.ascender)
            and pdf_integer(descriptor['/Descent']) == scaled(source.descender), 'number descriptor from source font metrics')
    return {'source_font_sha256':hashlib.sha256(source.raw).hexdigest(),
        'embedded_font_sha256':hashlib.sha256(subset.raw).hexdigest(),'units_per_em':source.units,
        'source_glyphs':len(rows),'subset_glyphs':subset.glyph_count,'cids':rows}


def reference_font(writer, source, characters, *, name='DejaVuSans'):
    """Embed unchanged original sfnt bytes, using original GIDs as reference CIDs."""
    from pypdf.generic import ArrayObject,DecodedStreamObject,DictionaryObject,NameObject,NumberObject,TextStringObject
    require(isinstance(name,str) and re.fullmatch(r'[A-Za-z][A-Za-z0-9]*',name), 'plain original reference font name')
    glyphs = sorted((source.codepoint(ord(char)),char) for char in set(characters))
    require(glyphs and len({gid for gid,_ in glyphs}) == len(glyphs), 'unique original reference glyphs')
    program = DecodedStreamObject();program.set_data(source.raw)
    program[NameObject('/Length1')] = NumberObject(len(source.raw))
    program_ref = writer._add_object(program.flate_encode())
    scaled = lambda value:NumberObject(int(Fraction(value*1000,source.units)))
    descriptor = DictionaryObject({NameObject('/Type'):NameObject('/FontDescriptor'),
        NameObject('/FontName'):NameObject('/'+name),NameObject('/Flags'):NumberObject(32),
        NameObject('/FontBBox'):ArrayObject([scaled(value) for value in source.bbox]),
        NameObject('/ItalicAngle'):NumberObject(0),NameObject('/Ascent'):scaled(source.ascender),
        NameObject('/Descent'):scaled(source.descender),NameObject('/CapHeight'):scaled(source.ascender),
        NameObject('/StemV'):NumberObject(80),NameObject('/FontFile2'):program_ref})
    widths = ArrayObject()
    for gid,char in glyphs:
        width=(source.metrics(gid)[0]*1000+source.units//2)//source.units
        widths.extend([NumberObject(gid),ArrayObject([NumberObject(width)])])
    descendant = DictionaryObject({NameObject('/Type'):NameObject('/Font'),NameObject('/Subtype'):NameObject('/CIDFontType2'),
        NameObject('/BaseFont'):NameObject('/'+name),NameObject('/CIDSystemInfo'):DictionaryObject({
            NameObject('/Registry'):TextStringObject('Adobe'),NameObject('/Ordering'):TextStringObject('Identity'),
            NameObject('/Supplement'):NumberObject(0)}),NameObject('/FontDescriptor'):writer._add_object(descriptor),
        NameObject('/DW'):NumberObject(1000),NameObject('/W'):widths,NameObject('/CIDToGIDMap'):NameObject('/Identity')})
    cmap = DecodedStreamObject()
    cmap.set_data(CMAP_HEAD+str(len(glyphs)).encode()+b' beginbfchar\n'+b''.join(
        f'<{gid:04X}> <{ord(char):04X}>\n'.encode() for gid,char in glyphs)+b'endbfchar\n'+CMAP_END)
    top = DictionaryObject({NameObject('/Type'):NameObject('/Font'),NameObject('/Subtype'):NameObject('/Type0'),
        NameObject('/BaseFont'):NameObject('/'+name),NameObject('/Encoding'):NameObject('/Identity-H'),
        NameObject('/DescendantFonts'):ArrayObject([writer._add_object(descendant)]),
        NameObject('/ToUnicode'):writer._add_object(cmap.flate_encode())})
    return writer._add_object(top)
