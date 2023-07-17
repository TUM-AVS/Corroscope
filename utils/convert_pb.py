from commonroad.common.util import FileFormat
from commonroad.common.file_reader import CommonRoadFileReader
from commonroad.common.file_writer import CommonRoadFileWriter

from pathlib import Path
import sys

if __name__ == '__main__':
    path = Path(sys.argv[1])

    if not path.exists():
        raise RuntimeError("file does not exist")

    print(f"converting {str(path)}")

    reader = CommonRoadFileReader(path, file_format=FileFormat.XML)
    scenario, planning_problem = reader.open()
    writer = CommonRoadFileWriter(scenario, planning_problem, file_format=FileFormat.PROTOBUF)
    writer.write_to_file()
