# Audio Engine

This is responsible for processing the correct entities in order of expected flow, essentially a scene graph for audio. Audio sources (synths, samples) can be added as leaf nodes to the graph, then modulated and blended before reaching the main output.

## Document Model

This mind map includes a directional flow of the levels of hierarchy inside of a Project:

<iframe style="border:none" width="800" height="450" src="https://whimsical.com/embed/HNvThW62PrN9Gp33UfAuRf@2Ux7TurymNHcs4gAfi4M"></iframe>

### Project

The highest level in the model. This includes filename, metadata, and a main output channel (ch0).

### Channel

Contains an FX Chain and gets audio from one or more DeviceTracks. Can also send audio output to one or more other Channels ([DAG](https://en.wikipedia.org/wiki/Directed_acyclic_graph)).

### DeviceTrack

Contains a Device and one or more state representations of that Device's MIDI sequence (like having multiple patterns for the same device in FL Studio).

### Device

A unit which produces sound. This can be an audio sampler which allows a sample to be loaded in and tweaked, it can be a simple sine wave oscillator, or it can be a complex VST instrument.
