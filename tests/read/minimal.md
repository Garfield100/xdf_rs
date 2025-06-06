This is a minimal example file with two streams.

# Stream 0

3 int16 channels, 9 samples

Data: 

```
[192, 255, 238],
[ 12,  22,  32],
[ 13,  23,  33],
[ 14,  24,  34],
[ 15,  25,  35],
[ 12,  22,  32],
[ 13,  23,  33],
[ 14,  24,  34],
[ 15,  25,  35]
```

Timestamps: 5.1 to 5.9 in .1 steps

The stream's XML header:
```
<info>
    <name>SendDataC</name>
    <type>EEG</type>
    <channel_count>3</channel_count>
    <nominal_srate>10</nominal_srate>
    <channel_format>int16</channel_format>
    <created_at>50942.723319709003</created_at>
    <desc/>
    <uid>xdfwriter_11_int</uid>
</info>
```
its footer:
```
<info>
    <writer>LabRecorder xdfwriter</writer>
    <first_timestamp>5.1</first_timestamp>
    <last_timestamp>5.9</last_timestamp>
    <sample_count>9</sample_count>
    <clock_offsets>
        <offset>
            <time>50979.76</time>
            <value>-.01</value>
        </offset>
        <offset>
            <time>50979.86</time>
            <value>-.02</value>
        </offset>
    </clock_offsets>
</info>
```


# Stream 0x02C0FFEE / 46202862

1 string channel, 9 samples

Data: `[ (the XML footer), 'Hello', 'World', 'from', 'LSL', 'Hello', 'World', 'from', 'LSL']`

Timestamps: as above

The stream's XML header:
```
<info>
    <name>SendDataString</name>
    <type>StringMarker</type>
    <channel_count>1</channel_count>
    <nominal_srate>10</nominal_srate>
    <channel_format>string</channel_format>
    <created_at>50942.723319709003</created_at>
    <desc/>
    <uid>xdfwriter_11_str</uid>
</info>
```
This stream's footer is identical to the previous one.