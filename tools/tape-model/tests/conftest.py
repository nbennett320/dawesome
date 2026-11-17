# whole passes of the test file through the fake deck, aligned and
# measured, made once per test run and shared: each takes about a minute

from dataclasses import replace
from functools import lru_cache

import pytest

from tape_model import align, faketape, measure, testfile


@lru_cache(maxsize=1)
def built():
  return testfile.build(seed=1)


# the 4-track layout: content on tracks 1 and 2, the pilot alone on 4
@lru_cache(maxsize=1)
def built4():
  return testfile.build(seed=1, pilot_track=3)


# the decks the criteria in context/tape-project/02-plan.md are about. the
# fresh and worn ones have a 0.7-frame delay on the right (azimuth)
DECKS = {
  "fresh": replace(faketape.preset("fresh"), channel_delay=0.7),
  "fresh+6": replace(faketape.preset("fresh"), channel_delay=0.7, drive_db=6.0),
  "fresh+18": replace(faketape.preset("fresh"), channel_delay=0.7, drive_db=18.0),
  "worn": replace(faketape.preset("worn"), channel_delay=0.7),
  "ancient": faketape.preset("ancient"),
  "print-through": replace(faketape.preset("fresh"), print_through_db=-34.0),
  # 4-track, pilot on its own (undriven) track, driven hard
  "4track+18": replace(faketape.preset("fresh"), channel_delay=0.7, drive_db=18.0, undriven_track=3),
}


class Pass:
  def __init__(self, name):
    test, test_map = built4() if name.startswith("4track") else built()
    self.name, self.deck, self.test, self.map = name, DECKS[name], test, test_map
    self.rec, self.truth = faketape.run(test, self.deck)
    self.alignment = align.align(self.rec, test_map.rate, test, test_map)
    self._measured = None

  def measured(self, reference=None):
    if self._measured is None:
      self._measured = measure.measure(self.alignment, self.alignment.recording, self.test, self.map, reference)
    return self._measured


@lru_cache(maxsize=None)
def deck_pass(name):
  return Pass(name)


@pytest.fixture(scope="session")
def passes():
  return deck_pass
